# Resolve the compiler before Cargo modifies PATH for build scripts.
CC := $(shell command -v clang)
AR := $(shell command -v llvm-ar)
LD := rust-lld
CARGO ?= cargo
CKB_DEBUGGER ?= ckb-debugger
PYTHON ?= python3
CASE ?= all
KZG_ARGS ?=
BLOB_TREE_FIXTURES ?= ../blob-tree/tests/fixtures/single.bin ../blob-tree/tests/fixtures/multirow.bin
BLOB_TREE_BENCH_FIXTURES ?= $(BLOB_TREE_FIXTURES) ../blob-tree/tests/fixtures/large.bin
MAX_CYCLES ?= 3500000000
BINARY := target/riscv64imac-unknown-none-elf/release/ckb-kzg-tests
BLOB_TREE_BINARY := target/riscv64imac-unknown-none-elf/release/blob-tree

.PHONY: build test test-montgomery test-kzg test-blob-tree bench-blob-tree clean
build:
	CC="$(CC)" CLANG="$(CC)" AR="$(AR)" $(CARGO) build --locked --release --bins

test: test-montgomery test-kzg test-blob-tree

test-montgomery: build
	$(CKB_DEBUGGER) --bin target/riscv64imac-unknown-none-elf/release/montgomery --max-cycles $(MAX_CYCLES)

test-kzg: build
	$(PYTHON) run.py --debugger "$(CKB_DEBUGGER)" --max-cycles $(MAX_CYCLES) kzg --binary $(BINARY) --case $(CASE) $(KZG_ARGS)

test-blob-tree: build
	$(PYTHON) run.py --debugger "$(CKB_DEBUGGER)" --max-cycles $(MAX_CYCLES) blob-tree --binary $(BLOB_TREE_BINARY) $(BLOB_TREE_FIXTURES)

bench-blob-tree: build
	$(PYTHON) run.py --debugger "$(CKB_DEBUGGER)" --max-cycles $(MAX_CYCLES) blob-tree --binary $(BLOB_TREE_BINARY) --benchmark $(BLOB_TREE_BENCH_FIXTURES)

clean:
	$(CARGO) clean
