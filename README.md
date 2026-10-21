[![Build and Test](https://github.com/ImperialBower/bowery/actions/workflows/ci.yaml/badge.svg)](https://github.com/ImperialBower/bowery/actions/workflows/ci.yaml)
[![Contributor Covenant](https://img.shields.io/badge/Contributor%20Covenant-2.0-4baaaa.svg)](CODE_OF_CONDUCT.md)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](LICENSE-MIT)
[![License: Apache 2.0](https://img.shields.io/badge/license-Apache%202.0-blue?style=flat-square)](LICENSE-APACHE)
[![AI-BOM](https://img.shields.io/badge/AI--BOM-declared-8A2BE2?style=flat-square)](AI-BOM.md)

---

# bowery

One poker table control, configured not compiled. `bowery` puts a poker table
in any web page as `<bowery-table>`: a strict replay of a recorded hand, an
interactive replay where the reader takes over any seat at any moment, or a
table against [pkcore](https://github.com/ImperialBower/pkcore)'s bots.
Seats, names, stacks, blinds, ante, button, deck, view and layout all come
from a YAML config in the page, and many tables can sit on one page. One Rust
crate builds two ways: an rlib kernel (config, check, game, frame, SVG
drawing) and, behind a `wasm` feature, the browser element. The design is
bower's [EPIC-20](https://github.com/ImperialBower/bower/blob/main/docs/EPIC-20_Bowery.md).

This repository is freshly scaffolded: the crate holds no table code yet.

## How to use

```sh
git clone https://github.com/ImperialBower/bowery.git
cd bowery
make install-tools   # cargo-deny, cargo-audit, cargo-watch
make ayce            # the full gate: clean, fmt, build, test, lint, security-scan, docs
```

`make help` lists every target.

## What's in the box

- **Toolchain:** Rust, edition 2024, MSRV pinned to pkcore's — see [toolchain decisions](.okf/decisions/toolchain.md)
- **Formatting:** rustfmt — see [toolchain decisions](.okf/decisions/toolchain.md)
- **Linting:** clippy at pedantic, warnings as errors — see [toolchain decisions](.okf/decisions/toolchain.md)
- **Supply-chain policy:** cargo-deny — see [toolchain decisions](.okf/decisions/toolchain.md)
- **Security scanning:** cargo-audit plus cargo-deny advisories, daily in CI — see [toolchain decisions](.okf/decisions/toolchain.md)
- **Docs:** rustdoc with warnings as errors — see [toolchain decisions](.okf/decisions/toolchain.md)
- **Quality gate:** `make ayce` — see [quality gate](.okf/processes/quality-gate.md)

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.
