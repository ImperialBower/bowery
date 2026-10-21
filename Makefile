.PHONY: default help clean fmt build wasm test lint security-scan docs ayce
default: ayce

help:  ## self-documenting: every target has a '## comment' printed here
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  make %-16s %s\n", $$1, $$2}'

clean: ## remove build artifacts
	cargo clean
fmt: ## format all sources
	cargo fmt
build: ## compile/build
	cargo build
wasm: ## build the wasm feature for the browser; keep wasm-bindgen out of the default tree
	cargo build --target wasm32-unknown-unknown --features wasm
	@if cargo tree -e normal | grep -q wasm-bindgen; then \
		echo "wasm-bindgen is in the default dependency tree"; exit 1; fi
test: ## run all tests
	cargo test
lint: ## static analysis at the pedantic end
	cargo clippy --all-targets -- -D warnings
	cargo deny check
security-scan: ## dependency vulnerability scan
	./bin/security-scan
docs: ## build docs, fail on warnings
	RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
ayce: clean fmt build wasm test lint security-scan docs ## all-you-can-eat: full pre-push sweep

.PHONY: watch install-tools

watch: ## rebuild/retest on save (requires cargo-watch)
	cargo watch -x "check --workspace" -x "test --workspace"

install-tools: ## install the wasm target, cargo-deny, cargo-audit, and cargo-watch
	rustup target add wasm32-unknown-unknown
	cargo install cargo-deny
	cargo install cargo-audit
	cargo install cargo-watch
