---
type: process
title: Release
description: How a bowery version goes out to crates.io, npm and a GitHub release tarball from one tag, and what to check first.
tags: [release, crates.io, npm, ci]
timestamp: '2026-10-22T00:00:00Z'
---

# Release

One tag `vX.Y.Z` publishes three things from one commit, in this order:
crates.io, then npm, then the GitHub release. The workflow is
`.github/workflows/release.yml`. Publishing cannot be undone, so a failure
part-way is fixed with a new patch version, never by deleting or retagging.

## Before tagging

1. Set the same version in `Cargo.toml` and `npm/package.json`, and add a
   `## [X.Y.Z] - date` section to `CHANGELOG.md`; its body becomes the
   release notes.
2. Run `make ayce`, and `make dist`, which runs the release checks locally.
   `cargo publish --dry-run` and `npm pack --dry-run ./dist/bowery` show what
   would ship.
3. Push, and wait for CI to be green on that commit.
4. The repository needs the secret `CARGO_REGISTRY_TOKEN`. npm needs no
   secret: on npmjs.com, `@imperialbower/bowery` → Settings → Trusted
   Publisher names GitHub Actions, `ImperialBower/bowery`, workflow
   `release.yml`. The npm job's `id-token: write` permission is what proves
   the publish comes from that workflow.

## Tag

`git tag vX.Y.Z && git push origin vX.Y.Z`.

## What ships where

- **crates.io:** the kernel and its tests. `exclude` in `Cargo.toml` leaves
  out the element, the demo and the browser proof.
- **npm and the tarball:** `dist/bowery/`, from `scripts/dist.sh`: the
  element, the glue, the wasm, the licences, the README, `package.json`,
  `MANIFEST.sha256`.

See the [quality gate](quality-gate.md).
