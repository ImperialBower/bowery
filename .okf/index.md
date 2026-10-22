---
okf_version: '0.1'
---

# bowery

* [Getting started](getting-started.md) - starting point for this bundle.
* [Toolchain](decisions/toolchain.md) - which tools the gate uses, and why.
* [Quality gate](processes/quality-gate.md) - what `make ayce` runs and what green means.
* [Release](processes/release.md) - one tag to crates.io, npm and a tarball; what to check first.
* [Config and check](concepts/config-and-check.md) - reading the author's YAML and refusing a config that is not true.
* [Game](concepts/game.md) - moments, going off the record, and how every hand's end comes from pkcore.
* [Frame and draw](concepts/frame-and-draw.md) - the view applied to a moment, geometry first, one SVG renderer and its golden files.
* [The element](concepts/element.md) - the Rust surface, the wasm wrapper, the JS element, the size gate and the demo.
* [Browser proof](concepts/browser-proof.md) - the Playwright suite, its pages, and making browser tests able to fail.
