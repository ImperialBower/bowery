---
type: process
title: Release
description: How a bowery version goes out to crates.io, npm and a GitHub release tarball from one tag, and what to check first.
tags: [release, crates.io, npm, ci]
timestamp: '2026-10-23T00:00:00Z'
---

# Release

One tag `vX.Y.Z` publishes three things from one commit, in this order:
crates.io, then npm, then the GitHub release. The workflow is
`.github/workflows/release.yml`. Publishing cannot be undone. A step that
failed before it published anything is fixed outside the repo, then
`gh run rerun <id> --failed` runs only the failed jobs: v0.1.0's npm step was
fixed that way. A fix inside the repo needs a new patch version; never delete
or move a tag.

## Before tagging

1. Set the same version in `Cargo.toml`, `npm/package.json` and the
   README's CDN link (`@imperialbower/bowery@X.Y.Z`), and add a
   `## [X.Y.Z] - date` section to `CHANGELOG.md`; its body becomes the
   release notes. `scripts/dist.sh` refuses any mismatch. The README link
   stays exact, never `@latest` or `@0.1`: `bowery.js` loads the glue and
   the wasm from beside it, and jsDelivr resolves a range per file and caches
   it for up to 7 days, so a range can serve them from two releases.
2. Run `make ayce`, and `make dist`, which runs the release checks locally.
   `cargo publish --dry-run` and `npm pack --dry-run ./dist/bowery` show what
   would ship.
3. Push, and wait for CI to be green on that commit.
4. No secrets. Both registries use trusted publishing. On crates.io, the
   crate's **Settings → Trusted Publishing** names GitHub `ImperialBower` /
   `bowery`, workflow `release.yml`, no environment; the crates.io job trades
   GitHub's OIDC token for a short-lived token through
   `rust-lang/crates-io-auth-action`. On npmjs.com, `@imperialbower/bowery` → Settings → Trusted
   Publisher names GitHub Actions, `ImperialBower/bowery`, workflow
   `release.yml`, no environment, and **Allow `npm publish`** ticked. Without
   that tick only `npm stage publish` is allowed, and the publish fails with
   "403 OIDC permission denied for this action". With no trusted publisher
   at all it fails with a 404. The npm job's `id-token: write` permission is
   what proves the publish comes from that workflow.

## Tag

`git tag vX.Y.Z && git push origin vX.Y.Z`.

## What ships where

- **crates.io:** the kernel and its tests. `exclude` in `Cargo.toml` leaves
  out the element, the demo and the browser proof.
- **npm and the tarball:** `dist/bowery/`, from `scripts/dist.sh`: the
  element, the glue, the wasm, the licences, the README, `package.json`,
  `MANIFEST.sha256`.

## The demo on Pages

`.github/workflows/pages.yaml` publishes `site/` (`make site`: the demo, the
element from this commit, configs and stills) to
<https://imperialbower.github.io/bowery/> on every push to `main`, not per
tag, after `make e2e` passes; `site/e2e/` is removed first. The demo loads
the element from `bowery/` beside it, never from the CDN, so it shows `main`,
which can be ahead of the latest release. It needs Settings → Pages → Source:
"GitHub Actions". Every path in `demo/index.html` is relative, because Pages
serves it under `/bowery/`.

See the [quality gate](quality-gate.md).
