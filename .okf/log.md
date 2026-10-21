# Update Log

## 2026-10-21
* **Creation**: Repository scaffolded by dev-playbook (rust); bundle seeded with `okf_init.py` — see [getting started](getting-started.md), [toolchain](decisions/toolchain.md), [quality gate](processes/quality-gate.md).
* **Update**: `make wasm` (EPIC-20 item 0d) joins the [quality gate](processes/quality-gate.md) and CI; `bin/security-scan` was hidden by a global excludes file, so `.gitignore` re-includes `/bin/`. `docs/EPIC-110_Bowery.md` points to bower EPIC-20 (item 0b).
