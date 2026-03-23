## hutc

Lua-driven HTTP API test runner built in Rust.

## Installation

Install from crates.io:

```bash
cargo install hutc
```

## Project Structure

```text
.
├── assets/
│   └── lua/
│       └── hutc.defs.lua      # LuaLS definition template used by `hutc init`
├── examples/
│   └── http_examples.lua      # usage examples
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
   - `hutc init`
2. Write tests in `lua/tests`.
3. Run tests:
   - `hutc test`

## Development

Run from source without installing:

```bash
cargo run -- test
```
