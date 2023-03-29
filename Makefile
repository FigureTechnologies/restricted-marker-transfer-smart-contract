#!/usr/bin/make -f
CONTAINER_RUNTIME := $(shell which docker 2>/dev/null || which podman 2>/dev/null)
UNAME_M := $(shell uname -m)

.PHONY: all
all: clean fmt lint test schema optimize

.PHONY: clean
clean:
	@cargo clean

.PHONY: fmt
fmt:
	@cargo fmt --all -- --check

.PHONY: lint
lint:
	@cargo clippy

.PHONY: build
build:
	@cargo build

.PHONY: test
test:
	@cargo test --verbose

.PHONY: schema
schema:
	@cargo run --example schema

.PHONY: coverage
coverage:
	@cargo tarpaulin --ignore-tests --out Html

.PHONY: optimize
optimize:
ifeq ($(UNAME_M),arm64)
	@docker run --rm -v $(CURDIR):/code \
		--mount type=volume,source=restricted-marker-transfer_cache,target=/code/target \
		--mount type=volume,source=restricted-marker-transfer_registry_cache,target=/usr/local/cargo/registry \
		cosmwasm/rust-optimizer-arm64:0.12.6
else
	@docker run --rm -v $(CURDIR):/code \
		--mount type=volume,source=restricted-marker-transfer_cache,target=/code/target \
		--mount type=volume,source=restricted-marker-transfer_registry_cache,target=/usr/local/cargo/registry \
		cosmwasm/rust-optimizer:0.12.6
endif

.PHONY: install
install: optimize
	@cp artifacts/restricted_marker_transfer.wasm $(PIO_HOME)
