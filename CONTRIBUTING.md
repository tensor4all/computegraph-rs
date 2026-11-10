<!-- unified contribution policy: ../../docs/contributing/rust-repositories.md in tensor4all/tensor4all-meta -->
# Contributing to computegraph-rs

The rules that apply to every Rust repository in `tensor4all` are in one place:
**[tensor4all-meta · docs/contributing/rust-repositories.md](https://github.com/tensor4all/tensor4all-meta/blob/main/docs/contributing/rust-repositories.md)**.
Read that first — it covers the issue-first workflow, what a pull request body has to contain, red-first tests, the
gates every repository runs, the review requirement, and the version policy.

This file only records what is specific to `computegraph-rs`:

## Local facts

- **MSRV**: `rust-version = "1.56"` in `Cargo.toml`. CI pins `stable` and has no `msrv` job, so the claim is not
  checked by CI.
- **Feature combinations CI builds**: one. The package declares no features, and both jobs run the default set.
- **Platforms claimed**: `ubuntu-latest` and `macos-latest`. Tests run through `cargo nextest run --workspace
  --release --no-fail-fast`, followed by `cargo test --doc --workspace --release`.
- **Local gate**: `cargo test --workspace --release`.

## Anything here that departs from the shared rules

Two do:

- There is no `clippy` job; the pipeline is `rustfmt`, `nextest` and doctests only. Run
  `cargo clippy --workspace --all-targets -- -D warnings` locally until a job exists.
- There is no `msrv` gate, and `rust-version = "1.56"` is far enough behind `stable` that it will not stay true by
  accident.

Both are gaps to close, not exemptions from the other gates.
