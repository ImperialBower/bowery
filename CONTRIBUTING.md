# Contributing to bowery

Thank you for your interest in contributing! Please take a moment to read these guidelines before submitting a pull request.

## Getting Started

1. Fork the repository and clone it locally.
2. Install the development tools (see the repo README for the toolchain pin).
3. Make your changes on a new branch.

## Development Workflow

**Non-negotiable: run `make ayce` before opening a PR — CI runs the same checks.**

```shell
make ayce
```

`ayce` is the default Makefile target and runs, in order: clean → format → build → test → lint → security-scan → docs. If it doesn't pass locally, it won't pass in CI.

Individual commands are also available — see `make help` for the full list.

For continuous feedback during development, use the project's watch target if one is provided.

## Code Standards

- Public APIs must be documented with working examples where the language ecosystem supports it.
- Tests are required for new behavior and bug fixes.
- Lint and formatting must be clean — `make ayce` enforces this.
- No secrets, credentials, or generated artifacts committed to the repository.

## Submitting a Pull Request

- Keep PRs focused — one feature or fix per PR.
- Fill out the pull request template.
- Ensure all CI checks pass.

## Reporting Issues

Use the issue templates under `.github/ISSUE_TEMPLATE/` if present.

## Code of Conduct

This project follows the [Contributor Covenant](CODE_OF_CONDUCT.md).
