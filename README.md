## hutc

Lua-driven HTTP API test runner built in Rust.

## Project Structure

```text
.
├── assets/
│   └── lua/
│       └── hutc.defs.lua      # LuaLS definition template used by `hutc init`
├── examples/
│   └── lua/
│       └── http_examples.lua  # usage examples
├── src/                       # Rust source code
├── Cargo.toml
└── README.md
```

## Commands

- `hutc test` runs all `.lua` tests under `tests`.
- `hutc test <path>` runs tests from a custom file or directory.
- `hutc init` generates `tests/hutc.defs.lua`.
- `hutc init <path>` generates `hutc.defs.lua` in a custom directory.

## Typical Setup

1. Generate definitions for Lua language server:
   - `cargo run -- init`
2. Write tests in `tests`.
3. Run tests:
   - `cargo run -- test`
