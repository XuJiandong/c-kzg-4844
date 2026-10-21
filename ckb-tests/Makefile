# Resolve the compiler before Cargo modifies PATH for build scripts.
CC := $(shell command -v clang)
AR := $(shell command -v llvm-ar)
LD := rust-lld
CARGO ?= cargo
CKB_DEBUGGER ?= ckb-debugger
CASE ?= all
MAX_CYCLES ?= 3500000000
BINARY := target/riscv64imac-unknown-none-elf/release/ckb-kzg-tests

.PHONY: build test clean
build:
	CC="$(CC)" CLANG="$(CC)" AR="$(AR)" $(CARGO) build --locked --release

test: build
	$(CKB_DEBUGGER) --bin $(BINARY) --max-cycles $(MAX_CYCLES) -- $(CASE)

clean:
	$(CARGO) clean
