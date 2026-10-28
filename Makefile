.PHONY: all
all: c csharp elixir go java nim nodejs python rust zig

.PHONY: c
c:
	@$(MAKE) -C src

.PHONY: csharp
csharp:
	@$(MAKE) -C bindings/csharp

.PHONY: elixir
elixir:
	@mix deps.get && mix test

.PHONY: go
go:
	@cd bindings/go && go clean -cache && go test

.PHONY: java
java:
	@$(MAKE) -C bindings/java build test

.PHONY: nim
nim:
	@cd bindings/nim && nim test

.PHONY: nodejs
nodejs:
	@$(MAKE) -C bindings/node.js

.PHONY: python
python:
	@$(MAKE) -C bindings/python

.PHONY: rust
rust:
	@cargo test --features generate-bindings
	@cargo bench --no-run
	@cd fuzz && cargo build

.PHONY: zig-check
zig-check:
	@submodule_hash=$$(git ls-tree HEAD blst | awk '{print $$3}'); \
	zon_hash=$$(grep -o 'supranational/blst/archive/[0-9a-f]*' build.zig.zon | grep -o '[0-9a-f]*$$'); \
	if [ "$$submodule_hash" != "$$zon_hash" ]; then \
		echo "Error: blst submodule ($$submodule_hash) and build.zig.zon ($$zon_hash) are out of sync"; \
		exit 1; \
	fi

.PHONY: zig
zig: zig-check
	@zig build test

.PHONY: ckb-build ckb-test
ckb-build:
	$(MAKE) -C ckb-tests build

ckb-test:
	$(MAKE) -C ckb-tests test

.PHONY: blob-tree-test
blob-tree-test:
	cargo test --manifest-path blob-tree/Cargo.toml --locked
	cargo test --manifest-path blob-tree/Cargo.toml --locked --no-default-features --features verification-only
	cargo run --manifest-path blob-tree/Cargo.toml --locked --example basic
	$(MAKE) -C ckb-tests test-blob-tree
