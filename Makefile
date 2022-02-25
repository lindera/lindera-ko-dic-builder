BIN_DIR ?= ./bin
LINDERA_KO_DIC_BUILDER_VERSION ?= $(shell cargo metadata --no-deps --format-version=1 | jq -r '.packages[] | select(.name=="lindera-ko-dic-builder") | .version')

.DEFAULT_GOAL := build

clean:
	cargo clean

format:
	cargo fmt

test:
	cargo test

build:
	cargo build --release

tag:
	git tag v$(LINDERA_KO_DIC_BUILDER_VERSION)
	git push origin v$(LINDERA_KO_DIC_BUILDER_VERSION)

publish:
ifeq ($(shell curl -s -XGET https://crates.io/api/v1/crates/lindera-ko-dic-builder  | jq -r '.versions[].num' | grep $(LINDERA_KO_DIC_BUILDER_VERSION)),)
	cargo package && cargo publish
endif
