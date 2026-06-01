# computegraph-rs

Operation-agnostic computation graph engine in Rust.

Provides graph construction, logical resolution,
physical materialization, SSA compilation, and evaluation.

Fully generic over `Operation: GraphOperation`; it never references specific
primitive operation sets.

## Part of the tensor4all v2 stack

```text
computegraph-rs  ← this crate
tidu-rs          ← AD graph transforms
tenferro-rs      ← concrete tensor primitives
```
