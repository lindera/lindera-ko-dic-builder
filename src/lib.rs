use std::collections::BTreeMap;
use std::fs::File;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::{fs, u32};

use bincode;
use byteorder::{LittleEndian, WriteBytesExt};
use csv::StringRecord;
use glob::glob;
use yada::builder::DoubleArrayBuilder;

use lindera_core::character_definition::{CharacterDefinitions, CharacterDefinitionsBuilder};
use lindera_core::dictionary_builder::DictionaryBuilder;
use lindera_core::error::LinderaErrorKind;
use lindera_core::file_util::read_utf8_file;
use lindera_core::unknown_dictionary::{UnknownDictionary, UnknownDictionaryEntry};
use lindera_core::user_dictionary::UserDictionary;
use lindera_core::word_entry::{WordEntry, WordId};
use lindera_core::LinderaResult;

pub struct KodicBuilder {}

impl KodicBuilder {
    pub fn new() -> Self {
        KodicBuilder {}
    }
}

impl DictionaryBuilder for KodicBuilder {
    fn build_dictionary(&self, input_dir: &Path, output_dir: &Path) -> LinderaResult<()> {
        fs::create_dir_all(&output_dir)
            .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?;

        let chardef = self.build_chardef(input_dir, output_dir).unwrap();
        self.build_unk(input_dir, &chardef, output_dir).unwrap();
        self.build_dict(input_dir, output_dir).unwrap();
        self.build_cost_matrix(input_dir, output_dir).unwrap();

        Ok(())
    }

    fn build_chardef(
        &self,
        input_dir: &Path,
        output_dir: &Path,
    ) -> LinderaResult<CharacterDefinitions> {
        println!("BUILD CHARDEF");
        let char_def_path = input_dir.join("char.def");
        let char_def = read_utf8_file(&char_def_path)?;
        let mut char_definitions_builder = CharacterDefinitionsBuilder::default();
        char_definitions_builder.parse(&char_def)?;
        let char_definitions = char_definitions_builder.build();

        let wtr_chardef_path = output_dir.join(Path::new("char_def.bin"));
        let mut wtr_chardef = io::BufWriter::new(
            File::create(wtr_chardef_path)
                .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?,
        );
        bincode::serialize_into(&mut wtr_chardef, &char_definitions)
            .map_err(|err| LinderaErrorKind::Serialize.with_error(anyhow::anyhow!(err)))?;
        wtr_chardef
            .flush()
            .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?;

        Ok(char_definitions)
    }

    fn build_unk(
        &self,
        input_dir: &Path,
        chardef: &CharacterDefinitions,
        output_dir: &Path,
    ) -> LinderaResult<()> {
        println!("BUILD UNK");
        let unk_data_path = input_dir.join("unk.def");
        let unk_data = read_utf8_file(&unk_data_path)?;
        let unknown_dictionary = parse_unk(&chardef.categories(), &unk_data)?;
        let wtr_unk_path = output_dir.join(Path::new("unk.bin"));
        let mut wtr_unk = io::BufWriter::new(
            File::create(wtr_unk_path)
                .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?,
        );
        bincode::serialize_into(&mut wtr_unk, &unknown_dictionary)
            .map_err(|err| LinderaErrorKind::Serialize.with_error(anyhow::anyhow!(err)))?;
        wtr_unk
            .flush()
            .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?;

        Ok(())
    }

    fn build_dict(&self, input_dir: &Path, output_dir: &Path) -> LinderaResult<()> {
        println!("BUILD DICT");

        let pattern = if let Some(path) = input_dir.to_str() {
            format!("{}/*.csv", path)
        } else {
            return Err(
                LinderaErrorKind::Io.with_error(anyhow::anyhow!("Failed to convert path to &str."))
            );
        };

        let mut filenames: Vec<PathBuf> = Vec::new();
        for entry in
            glob(&pattern).map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?
        {
            match entry {
                Ok(path) => {
                    if let Some(filename) = path.file_name() {
                        filenames.push(Path::new(input_dir).join(filename));
                    } else {
                        return Err(LinderaErrorKind::Io
                            .with_error(anyhow::anyhow!("failed to get filename")));
                    };
                }
                Err(err) => return Err(LinderaErrorKind::Content.with_error(anyhow::anyhow!(err))),
            }
        }

        let mut rows: Vec<StringRecord> = Vec::new();
        for filename in filenames {
            let mut rdr = csv::ReaderBuilder::new()
                .has_headers(false)
                .from_path(filename)
                .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?;

            for result in rdr.records() {
                let record = result
                    .map_err(|err| LinderaErrorKind::Content.with_error(anyhow::anyhow!(err)))?;
                rows.push(record);
            }
        }

        println!("sorting entries");
        rows.sort_by_key(|row| row[0].to_string().clone());

        let wtr_da_path = output_dir.join(Path::new("dict.da"));
        let mut wtr_da = io::BufWriter::new(
            File::create(wtr_da_path)
                .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?,
        );

        let wtr_vals_path = output_dir.join(Path::new("dict.vals"));
        let mut wtr_vals = io::BufWriter::new(
            File::create(wtr_vals_path)
                .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?,
        );

        let mut word_entry_map: BTreeMap<String, Vec<WordEntry>> = BTreeMap::new();

        for (row_id, row) in rows.iter().enumerate() {
            word_entry_map
                .entry(row[0].to_string())
                .or_insert_with(Vec::new)
                .push(WordEntry {
                    word_id: WordId(row_id as u32, true),
                    word_cost: i16::from_str(row[3].trim()).map_err(|_err| {
                        LinderaErrorKind::Parse
                            .with_error(anyhow::anyhow!("failed to parse word_cost"))
                    })?,
                    cost_id: u16::from_str(row[1].trim()).map_err(|_err| {
                        LinderaErrorKind::Parse
                            .with_error(anyhow::anyhow!("failed to parse cost_id"))
                    })?,
                });
        }

        let wtr_words_path = output_dir.join(Path::new("dict.words"));
        let mut wtr_words = io::BufWriter::new(
            File::create(wtr_words_path)
                .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?,
        );

        let wtr_words_idx_path = output_dir.join(Path::new("dict.wordsidx"));
        let mut wtr_words_idx = io::BufWriter::new(
            File::create(wtr_words_idx_path)
                .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?,
        );

        let mut words_buffer = Vec::new();
        for row in rows.iter() {
            let word = vec![
                row[4].to_string(),
                row[5].to_string(),
                row[6].to_string(),
                row[7].to_string(),
                row[8].to_string(),
                row[9].to_string(),
                row[10].to_string(),
                row[11].to_string(),
            ];

            let offset = words_buffer.len();
            wtr_words_idx
                .write_u32::<LittleEndian>(offset as u32)
                .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?;
            bincode::serialize_into(&mut words_buffer, &word)
                .map_err(|err| LinderaErrorKind::Serialize.with_error(anyhow::anyhow!(err)))?;
        }

        wtr_words
            .write_all(&words_buffer[..])
            .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?;
        wtr_words
            .flush()
            .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?;
        wtr_words_idx
            .flush()
            .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?;

        let mut id = 0u32;

        println!("building da");
        let mut keyset: Vec<(&[u8], u32)> = vec![];
        let mut lastlen = 0;
        for (key, word_entries) in &word_entry_map {
            let len = word_entries.len() as u32;
            let val = (id << 5) | len;
            keyset.push((key.as_bytes(), val));
            id += len;
            lastlen += len;
        }

        let da_bytes = DoubleArrayBuilder::build(&keyset).ok_or_else(|| {
            LinderaErrorKind::Io.with_error(anyhow::anyhow!("DoubleArray build error."))
        })?;

        wtr_da
            .write_all(&da_bytes[..])
            .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?;
        println!("Last len is {}", lastlen);

        println!("building values");
        for word_entries in word_entry_map.values() {
            for word_entry in word_entries {
                word_entry
                    .serialize(&mut wtr_vals)
                    .map_err(|err| LinderaErrorKind::Serialize.with_error(anyhow::anyhow!(err)))?;
            }
        }
        wtr_vals
            .flush()
            .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?;

        Ok(())
    }

    fn build_cost_matrix(&self, input_dir: &Path, output_dir: &Path) -> LinderaResult<()> {
        println!("BUILD COST MATRIX");
        let matrix_data_path = input_dir.join("matrix.def");
        let matrix_data = read_utf8_file(&matrix_data_path)?;
        let mut lines = Vec::new();
        for line in matrix_data.lines() {
            let fields: Vec<i32> = line
                .split_whitespace()
                .map(i32::from_str)
                .collect::<Result<_, _>>()
                .map_err(|err| LinderaErrorKind::Parse.with_error(anyhow::anyhow!(err)))?;
            lines.push(fields);
        }
        let mut lines_it = lines.into_iter();
        let header = lines_it.next().ok_or_else(|| {
            LinderaErrorKind::Content.with_error(anyhow::anyhow!("unknown error"))
        })?;
        let forward_size = header[0] as u32;
        let backward_size = header[1] as u32;
        let len = 2 + (forward_size * backward_size) as usize;
        let mut costs = vec![i16::max_value(); len];
        costs[0] = forward_size as i16;
        costs[1] = backward_size as i16;
        for fields in lines_it {
            let forward_id = fields[0] as u32;
            let backward_id = fields[1] as u32;
            let cost = fields[2] as u16;
            costs[2 + (backward_id + forward_id * backward_size) as usize] = cost as i16;
        }

        let wtr_matrix_mtx_path = output_dir.join(Path::new("matrix.mtx"));
        let mut wtr_matrix_mtx = io::BufWriter::new(
            File::create(wtr_matrix_mtx_path)
                .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?,
        );
        for cost in costs {
            wtr_matrix_mtx
                .write_i16::<LittleEndian>(cost)
                .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?;
        }
        wtr_matrix_mtx
            .flush()
            .map_err(|err| LinderaErrorKind::Io.with_error(anyhow::anyhow!(err)))?;

        Ok(())
    }

    fn build_user_dict(&self, input_file: &Path) -> LinderaResult<UserDictionary> {
        unimplemented!()
    }
}

// TODO remove if release https://github.com/lindera-morphology/lindera/pull/118
fn parse_dictionary_entry(fields: &[&str]) -> LinderaResult<UnknownDictionaryEntry> {
    if fields.len() != 12 {
        return Err(LinderaErrorKind::Content.with_error(anyhow::anyhow!(
            "Invalid number of fields. Expect 12, got {}",
            fields.len()
        )));
    }
    let surface = fields[0];
    let left_id = u32::from_str(fields[1])
        .map_err(|err| LinderaErrorKind::Parse.with_error(anyhow::anyhow!(err)))?;
    let right_id = u32::from_str(fields[2])
        .map_err(|err| LinderaErrorKind::Parse.with_error(anyhow::anyhow!(err)))?;
    let word_cost = i32::from_str(fields[3])
        .map_err(|err| LinderaErrorKind::Parse.with_error(anyhow::anyhow!(err)))?;
    Ok(UnknownDictionaryEntry {
        surface: surface.to_string(),
        left_id,
        right_id,
        word_cost,
    })
}

fn make_costs_array(entries: &[UnknownDictionaryEntry]) -> Vec<WordEntry> {
    entries
        .iter()
        .map(|e| {
            // the following `assert_eq!()` is ignored because `left_id` and `right_id` registered
            // in `unk.def` are not the same.
            //assert_eq!(e.left_id, e.right_id);
            WordEntry {
                word_id: WordId(std::u32::MAX, true),
                cost_id: e.left_id as u16,
                word_cost: e.word_cost as i16,
            }
        })
        .collect()
}

fn get_entry_id_matching_surface(
    entries: &[UnknownDictionaryEntry],
    target_surface: &str,
) -> Vec<u32> {
    entries
        .iter()
        .enumerate()
        .filter_map(|(entry_id, entry)| {
            if entry.surface == target_surface.to_string() {
                Some(entry_id as u32)
            } else {
                None
            }
        })
        .collect()
}

fn make_category_references(
    categories: &[String],
    entries: &[UnknownDictionaryEntry],
) -> Vec<Vec<u32>> {
    categories
        .iter()
        .map(|category| get_entry_id_matching_surface(entries, category))
        .collect()
}

fn parse_unk(categories: &[String], file_content: &String) -> LinderaResult<UnknownDictionary> {
    let mut unknown_dict_entries = Vec::new();
    for line in file_content.lines() {
        let fields: Vec<&str> = line.split(",").collect::<Vec<&str>>();
        let entry = parse_dictionary_entry(&fields[..])?;
        unknown_dict_entries.push(entry);
    }

    let category_references = make_category_references(categories, &unknown_dict_entries[..]);
    let costs = make_costs_array(&unknown_dict_entries[..]);
    Ok(UnknownDictionary {
        category_references,
        costs,
    })
}
