# AI Bill of Materials — bowery

This file declares how AI assistance is used in this repository, so
consumers and contributors can see it at a glance. Keep it current: when a
new AI tool or model starts contributing, add a row; when one stops
contributing, note that too.

## AI tools in use

| Tool | Model | Role |
|---|---|---|
| Claude Code | Claude Opus 5.5 (`claude-opus-5-5`) | scaffolding |

## Provenance & oversight

- All AI-assisted changes must pass the quality gate (`make ayce`) before
  they are considered done — the same bar as human-written code.
- A human reviews, commits, and pushes every change; AI agents never run
  state-changing git commands in this repository's workflow.
- The coding standards AI agents follow live in [CLAUDE.md](CLAUDE.md).

## Scope

AI assistance may touch any part of the repository unless noted here.
Record exceptions (files or areas that must remain human-authored) below:

- _none declared_
