# Lindera ko-dic Builder

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT) [![Join the chat at https://gitter.im/lindera-morphology/lindera](https://badges.gitter.im/lindera-morphology/lindera.svg)](https://gitter.im/lindera-morphology/lindera?utm_source=badge&utm_medium=badge&utm_campaign=pr-badge&utm_content=badge)

ko-dic dictionary builder for [Lindera](https://github.com/lindera-morphology/lindera). This project fork from fulmicoton's [kuromoji-rs](https://github.com/fulmicoton/kuromoji-rs).

## Install

```
% cargo install lindera-ko-dic-builder
```

## Build

The following products are required to build:

- Rust >= 1.39.0
- make >= 3.81

```text
% make lindera-ko-dic
```

## Dictionary version

This repository contains [mecab-ko-dic-2.1.1-20180720](https://bitbucket.org/eunjeon/mecab-ko-dic/downloads/).

## Building a dictionary

Building a dictionary with `lindera-ko-dic` command:

```
% ./bin/lindera-ko-dic ./mecab-ko-dic-2.1.1-20180720 ./lindera-ko-dic-2.1.1-20180720
```

## Tokenizing text using produced dictionary

You can tokenize text using produced dictionary with `lindera` command:

```
% echo "하네다공항한정토트백" | lindera -d ./lindera-ko-dic-2.1.1-20180720                                  [~/github.com/lindera-morphology/lindera-ko-dic-builder]
하네다  NNP,인명,F,하네다,*,*,*,*,
공항    NNG,장소,T,공항,*,*,*,*,
한정    NNG,*,T,한정,*,*,*,*,
토트백  NNG,*,T,토트백,Compound,*,*,토트/NNP/인명+백/NNG/*,
EOS
```

## API reference

The API reference is available. Please see following URL:
- <a href="https://docs.rs/lindera-ipadic-builder" target="_blank">lindera-ko-dic-builder</a>
