---
type: concept
title: Browser proof
description: How bowery's Playwright tests are laid out and run, and the lessons about making them able to fail.
tags: [playwright, e2e, testing, epic-20]
timestamp: '2026-10-22T00:00:00Z'
---

# Browser proof

`tests/e2e/`, run by `make e2e`. It builds on [the element](element.md).

- **Own package.** `tests/e2e/package.json` pins `@playwright/test`
  (1.59.1, the version pkarena0-web has installed, so the browsers are
  cached). The repo root's `package.json` is kept for the npm package.
- **Pages.** `tests/e2e/pages/*.html` are copied to `site/e2e/`, and their
  configs are generated there from `tests/hands/` and `fixtures/` by
  `make e2e-pages`. `make site` deletes `site/`, so run `e2e-pages` after it.
- **Check every frame in the page.** Bots move every 25 ms. A test that
  samples on a timer misses frames, and passed a deliberately broken build.
  `checker.ts` hooks `bowery:moment` with `page.addInitScript` instead.
- **Prove a test can fail.** Break the build on purpose (move a box, drop a
  rule), rebuild with `make site e2e-pages`, run the spec, and undo the
  break. Check with `git diff` that the break is gone.
- **Custom elements are inline** until their stylesheet loads: measure only
  after `:host { display: block }` applies (inline `<style>` in the shadow
  root).
- `file://`: the browser refuses `type="module"` scripts there, so the
  element never upgrades; the fallback is what shows.
