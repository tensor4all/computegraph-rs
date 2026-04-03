# Computegraph Engine Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement the full generic computation graph engine, including graph construction, resolution, materialization, compilation, evaluation, and end-to-end integration tests.

**Architecture:** Build the crate as a small set of focused modules with a strict dependency flow: traits and structural key types at the bottom, fragment construction above them, resolution/materialization over fragments, and compilation/evaluation over the flattened graph. Drive the implementation through integration tests first so the public API and end-to-end behavior are locked before production code is added.

**Tech Stack:** Rust 2021, standard library only, `cargo fmt`, `cargo clippy`, `cargo test --release`

---

### Task 1: Add the public integration tests

**Files:**
- Create: `tests/common/mod.rs`
- Create: `tests/scalar_tests.rs`

**Step 1: Write the failing tests**

Add the shared scalar test helpers and the integration suite covering the interner, fragment builder, resolver, materializer, compiler, evaluator, and end-to-end graph execution.

**Step 2: Run test to verify it fails**

Run: `cargo test --release scalar_tests`
Expected: FAIL because the crate exports and modules do not exist yet.

**Step 3: Commit**

```bash
cargo fmt --all
git add docs/plans/2026-04-03-computegraph-engine.md tests/common/mod.rs tests/scalar_tests.rs
git commit -m "test: add computegraph integration coverage"
```

### Task 2: Implement the core library modules

**Files:**
- Create: `src/traits.rs`
- Create: `src/types.rs`
- Create: `src/interner.rs`
- Create: `src/fragment.rs`
- Create: `src/resolve.rs`
- Create: `src/materialize.rs`
- Create: `src/compile.rs`
- Create: `src/eval.rs`
- Modify: `src/lib.rs`

**Step 1: Write minimal implementation**

Implement the generic traits, key types, interner, fragment builder, resolver, graph materializer, compiler, and evaluator to satisfy the integration tests while keeping files small and avoiding `unwrap()` / `expect()` in library code.

**Step 2: Run focused tests**

Run: `cargo test --release scalar_tests`
Expected: PASS

**Step 3: Commit**

```bash
cargo fmt --all
git add src/lib.rs src/traits.rs src/types.rs src/interner.rs src/fragment.rs src/resolve.rs src/materialize.rs src/compile.rs src/eval.rs
git commit -m "feat: implement computegraph engine"
```

### Task 3: Full verification and cleanup

**Files:**
- Verify: `src/*.rs`
- Verify: `tests/common/mod.rs`
- Verify: `tests/scalar_tests.rs`

**Step 1: Run full verification**

Run:
- `cargo fmt --all`
- `cargo clippy --workspace`
- `cargo test --release`

Expected: all commands succeed and the full test suite passes.

**Step 2: Commit final cleanup if needed**

```bash
git add .
git commit -m "chore: finalize computegraph implementation"
```
