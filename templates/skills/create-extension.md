---
name: create-extension
description: Guide for scaffolding and compiling Rust extensions against a4l-ext-api.
when_to_load: user asks to build, compile, or install an extension
---

# Creating an A4L Extension

Extensions are Rust crates compiled into the user's binary.

## Structure
1. Depend on `a4l-ext-api`.
2. Implement the `Extension` trait:
```rust
use a4l_ext_api::{Extension, Registry};

pub struct MyExtension;

impl Extension for MyExtension {
    fn name(&self) -> &str { "my-extension" }
    fn version(&self) -> &str { "0.1.0" }
    fn register(&self, registry: &mut Registry) {
        // Register custom tools, lifecycle hooks, or channel adapters
    }
}
```

## Build and Install
1. Run `a4l ext add <path-or-repo>`.
2. Review dependency diff during HITL prompt.
3. Test with `a4l` or launch with `a4l --no-extensions` for clean mode.
