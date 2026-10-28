# Resolve the compiler before Cargo modifies PATH for build scripts.
CC := $(shell command -v clang)
AR := $(shell command -v llvm-ar)
LD := rust-lld
CARGO ?= cargo
CKB_DEBUGGER ?= ckb-debugger
CASE ?= all
MAX_CYCLES ?= 3500000000
BINARY := target/riscv64imac-unknown-none-elf/release/ckb-kzg-tests
BLOB_TREE_BINARY := target/riscv64imac-unknown-none-elf/release/blob-tree

.PHONY: build test test-kzg test-blob-tree clean
build:
	CC="$(CC)" CLANG="$(CC)" AR="$(AR)" $(CARGO) build --locked --release --bins

test: test-kzg test-blob-tree

test-kzg: build
	$(CKB_DEBUGGER) --bin $(BINARY) --max-cycles $(MAX_CYCLES) -- $(CASE)

test-blob-tree: build
	$(CKB_DEBUGGER) --bin $(BLOB_TREE_BINARY) --max-cycles $(MAX_CYCLES)

clean:
	$(CARGO) clean
