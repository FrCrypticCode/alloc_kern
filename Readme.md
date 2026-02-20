This project provides a minimal physical memory allocator for early‑stage kernel development in Rust (no_std).
It implements a bitmap‑driven first‑fit strategy with offset optimization and strictly encapsulates its internal state.
Only a small public API is exposed (constructor + physical allocate/free).
Virtual memory management will be added in a later stage, but the allocator is already usable on its own as a standalone component.