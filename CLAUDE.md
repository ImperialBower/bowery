# CLAUDE.md

## Definition of done

`make ayce` passes. Run it before claiming any work complete.

## Knowledge bundle

Consult `.okf/` for context; write durable learnings back to it (concepts need
`type` frontmatter; append to `.okf/log.md`).

## Version consistency

`.tool-versions`, `Cargo.toml`'s `rust-version`, `.github/workflows/ci.yaml`'s
`test` matrix MSRV entry, and `.devcontainer/devcontainer.json`'s
`postCreateCommand` (`rustup install/default <version>`; the image tag does not
encode the Rust version) MUST agree; whenever any one changes, verify all of
them. The MSRV tracks pkcore's `rust-version`.

## Design

The design lives in bower's `docs/EPIC-20_Bowery.md` (and EPIC-21 for the
bower block). bowery calls pkcore's `HandHistory::validate`, `cursor`,
`stacked_deck` and `start_hand_with_deck` and never re-implements them.


### Rust: Testing Requirements

**Unit Tests**
- Every public function must have at least one unit test covering the
  happy path.
- Every public struct/enum must have tests that validate construction,
  methods, and trait implementations.
- Place unit tests in a `#[cfg(test)]` module at the end of the file, or
  in a `tests/` directory for integration tests.
- Name tests descriptively: `<function_or_struct_name>_<scenario>`.
- Cover edge cases, error conditions, and boundary conditions.

**Doc Tests**
- Every public function and method must include at least one doc test
  (in the `///` doc comment, triple-backtick `rust` block) demonstrating
  basic usage.
- Doc tests must compile and run successfully — they are exercised by
  `cargo test` (no separate invocation needed).
- Show the most common usage pattern; if a function can fail, show a
  success-case example.

### Rust: Documentation Requirements

- **Functions**: single-sentence summary first line; explain what/why/when
  and caveats; document parameters, return value, `# Errors` (all failure
  cases), `# Panics` (if it can panic), `# Examples` (working doc test),
  and `# Safety` (if `unsafe`, explaining why and how to use safely).
- **Structs/enums**: purpose and role; document each field's meaning and
  invariants; show construction/usage examples; explain non-obvious trait
  behavior.
- **Modules**: start each file with a module-level `//!` doc comment
  explaining its purpose, plus common-usage examples.
- **Crate root (`lib.rs`)**: comprehensive overview, module-organization
  links, a quick-start guide, and complete working examples.

### Rust: Error Handling

- Never use `unwrap()`, `expect()`, or `panic!()` in library code. Tests
  may use them.
- Prefer `Result<T, E>` over `Option<T>` when failure carries meaningful
  information; create custom error types for domain-specific errors and
  implement `std::error::Error` for them.
- Document all error cases; use `?` for error propagation in library code.

### Rust: Naming

- `snake_case` for functions, variables, and modules; `PascalCase` for
  types, structs, enums, and traits.
- Avoid single-letter names except loop indices (`i`, `j`, `k`); prefer
  full words (`cards` not `c`, `rank` not `r`).

### Rust: Trait Guidance

- Implement `Display` for user-facing types.
- Implement `Debug` for all public types.
- Implement `Default` for types with a sensible default.
- Implement `Clone`/`Copy` when semantically appropriate.
- Document non-obvious trait behavior at the impl site.

### Rust: Code Organization

- Keep functions focused and single-purpose.
- Extract complex logic into well-named helper functions.
- Group related functions and types in logical modules.
- Use visibility modifiers (`pub`, `pub(crate)`, private) appropriately.

### Rust: Other Guidelines

- Prefer `&T` over `T` when a borrow suffices; use `&mut T` only when
  mutation is needed; document borrowing/lifetime requirements for
  non-trivial signatures.
- Avoid unnecessary cloning; prefer `&[T]` over `&Vec<T>` for slice
  parameters; pre-allocate with `with_capacity()` when the size is known;
  document performance characteristics of expensive operations.
- Prefer type-safe abstractions and enums over primitives/strings for
  fixed value sets; let the type system rule out invalid states.

