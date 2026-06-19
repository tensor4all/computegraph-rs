# Agent Guidelines for Rust Projects

Read `README.md` before starting work.

## Shared Tensor4all Rules

Cross-repository agent rules live in
[`tensor4all/tensor4all-agent-rules`](https://github.com/tensor4all/tensor4all-agent-rules).
Prefer the latest online version; if offline, use a sibling checkout at
`../tensor4all-agent-rules`. Read `rules/index.md` first, then load only the
rule files relevant to the task. The guidelines below are computegraph-specific
and take precedence for this repository.

## General Guidelines

- Always think/reason in English (set thinking language to English)
- Source code and docs in English
- **Bug fixing**: When a bug is discovered, always check related files for similar bugs and propose to the user to inspect them

## Code Style

`cargo fmt` for formatting, `cargo clippy` for linting. Avoid `unwrap()`/`expect()` in library code.

**Always run `cargo fmt --all` before committing changes.**

### File Organization

Keep source files **small and focused** — one logical concern per file.

## Error Handling

- Use `assert!` with informative messages for programming invariants

## Testing

**Always use `--release` mode for tests.**

```bash
cargo test --release              # Full suite
cargo test --doc --release        # Doc tests
```

- Private functions: `#[cfg(test)]` module in source file
- Integration tests: `tests/` directory

## Documentation

Public API doc comments (`///`) must include a minimal but sufficient example showing how to use the API.

## API Design

Only make functions `pub` when truly public API.
