BIN_DIR ?= ./bin
KO_DIC_VERSION ?= 2.1.1-20180720
SOURCE_URL ?= https://bitbucket.org/eunjeon/mecab-ko-dic/downloads/mecab-ko-dic-$(KO_DIC_VERSION).tar.gz
LINDERA_KO_DIC_BUILDER_VERSION ?= $(shell cargo metadata --no-deps --format-version=1 | jq -r '.packages[] | select(.name=="lindera-ko-dic-builder") | .version')

.DEFAULT_GOAL := build

clean:
	rm -rf $(BIN_DIR)
	rm -rf ./lindera-ko-dic-*
	rm -rf ./mecab-ko-dic-*
	cargo clean

format:
	cargo fmt

build:
	cargo build --release
	mkdir -p $(BIN_DIR)
	cp -p ./target/release/lindera-ko-dic $(BIN_DIR)

mecab-ko-dic-download:
ifeq ($(wildcard ./mecab-ko-dic-$(KO_DIC_VERSION).tar.gz),)
	curl -L -O "$(SOURCE_URL)"
endif

mecab-ko-dic-extract: mecab-ko-dic-download
ifeq ($(wildcard ./mecab-ko-dic-$(KO_DIC_VERSION)/*),)
	tar zxvf ./mecab-ko-dic-$(KO_DIC_VERSION).tar.gz
endif

lindera-ko-dic: build mecab-ko-dic-extract
	$(BIN_DIR)/lindera-ko-dic ./mecab-ko-dic-$(KO_DIC_VERSION) ./lindera-ko-dic-$(KO_DIC_VERSION)
	tar -cvjf ./lindera-ko-dic-$(KO_DIC_VERSION).tar.bz2 ./lindera-ko-dic-$(KO_DIC_VERSION)

test:
	cargo test

tag:
	git tag v$(LINDERA_KO_DIC_BUILDER_VERSION)
	git push origin v$(LINDERA_KO_DIC_BUILDER_VERSION)

publish:
ifeq ($(shell cargo show --json lindera-ko-dic-builder | jq -r '.versions[].num' | grep $(LINDERA_KO_DIC_BUILDER_VERSION)),)
	cargo package && cargo publish
endif
