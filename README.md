# computegraph-rs

AD-agnostic tensor computation graph engine in Rust.

Provides fragment-based graph construction, logical resolution,
physical materialization, SSA compilation, and evaluation.

Fully generic over `Op: GraphOp` — never references specific primitives.

## Part of the tensor4all v2 stack

```text
computegraph-rs  ← this crate
chainrules-rs    ← AD trait definitions
tidu-rs          ← AD graph transforms
tenferro-rs      ← concrete tensor primitives
```
