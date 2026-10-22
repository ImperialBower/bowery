.PHONY: default help clean fmt build wasm test lint security-scan docs ayce pkg size site serve
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
test: ## run all tests, the browser surface's included
	cargo test --features wasm
lint: ## static analysis at the pedantic end
	cargo clippy --all-targets --features wasm -- -D warnings
	cargo deny check
security-scan: ## dependency vulnerability scan
	./bin/security-scan
docs: ## build docs, fail on warnings
	RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --features wasm
ayce: clean fmt build wasm test lint security-scan docs size ## all-you-can-eat: full pre-push sweep

.PHONY: watch install-tools

watch: ## rebuild/retest on save (requires cargo-watch)
	cargo watch -x "check --workspace" -x "test --workspace"

install-tools: ## install the wasm target, cargo-deny, cargo-audit, and cargo-watch
	rustup target add wasm32-unknown-unknown
	cargo install cargo-deny
	cargo install cargo-audit
	cargo install cargo-watch

# The release wasm, measured on 22 October 2026 at 1,295,901 bytes; the gate
# is that plus 10% (EPIC-20 4a).
WASM_GATE := 1425491

pkg: ## build the browser module into pkg/ (wasm-pack, release, wasm-opt -Oz)
	wasm-pack build --release --target web --out-dir pkg --out-name bowery_wasm --no-typescript --no-pack -- --features wasm

size: pkg ## fail if the wasm is over the size gate
	@bytes=$$(wc -c < pkg/bowery_wasm_bg.wasm | tr -d ' '); \
		echo "bowery_wasm_bg.wasm: $$bytes bytes (gate $(WASM_GATE))"; \
		test $$bytes -le $(WASM_GATE)

site: pkg ## assemble the demo in site/: the element in site/bowery/, configs and stills beside it
	rm -rf site && mkdir -p site/bowery
	cp js/bowery.js js/bowery.css pkg/bowery_wasm.js pkg/bowery_wasm_bg.wasm LICENSE-MIT LICENSE-APACHE site/bowery/
	cp demo/index.html demo/the-hand-play.yaml site/
	{ printf 'mode: replay\nlayout: ring\nhand:\n'; sed 's/^/  /' fixtures/the_hand.yaml; } > site/the-hand-replay.yaml
	{ printf 'mode: interactive\nlayout: spotlight\nseat: 4\nstart: flop\nhand:\n'; sed 's/^/  /' fixtures/the_hand.yaml; } > site/the-hand-interactive.yaml
	cargo run -q --example bowery-still -- site/the-hand-replay.yaml --at flop > site/the-hand-flop.svg
	cargo run -q --example bowery-still -- site/the-hand-interactive.yaml --at flop > site/the-hand-spotlight.svg
	cargo run -q --example bowery-still -- demo/the-hand-play.yaml --view all > site/the-hand-play.svg

serve: site ## serve the demo at http://localhost:8000
	python3 -m http.server -d site 8000
