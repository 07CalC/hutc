## hutc

Lua-driven HTTP API test runner built in Rust.

## Installation

Install from crates.io:

```bash
cargo install hutc
```

## Quick Start

1. Generate Lua language-server definitions:

```bash
hutc init
```

This creates `tests/hutc.defs.lua`.

2. Create a test file at `tests/health.lua`:

```lua
local client = http()
client:base_url("https://example.com")

test("health endpoint is up", function()
  local res = client:req():path("/health"):get()

  expect(res.status):to_equal(200)
  expect(res.ok):to_equal(true)
end)
```

3. Run tests:

```bash
hutc test
```

## Default Paths

- `hutc test` reads `.lua` files from `tests`
- `hutc init` writes `tests/hutc.defs.lua`

## Lua API

### Globals

| Function | Purpose |
|---|---|
| `test(name, fn)` | Registers a test case |
| `expect(value)` | Creates an assertion object |
| `http()` | Creates an HTTP client |
| `log(...)` | Prints debug values |

### Assertions

`expect(value)` returns an `Expect` object:

| Method | Description |
|---|---|
| `:msg("message")` | Adds custom message prefix on assertion failure |
| `:to_equal(expected)` | Asserts equality |
| `:to_not_equal(expected)` | Asserts inequality |
| `:to_exist()` | Asserts value is not `nil` |

Example:

```lua
expect(res.status):msg("status mismatch"):to_equal(200)
expect(res.json.user):to_exist()
```

### HTTP Client

Create a client:

```lua
local client = http()
client:base_url("https://api.example.com")
```

Create a request with `client:req()`, then chain request-builder methods:

| Method | Description |
|---|---|
| `:path("/users")` | Relative path (uses `base_url`) |
| `:url("https://...")` | Absolute URL (overrides path/base_url) |
| `:header("k", "v")` | Single header |
| `:headers({ k = "v" })` | Multiple headers |
| `:query("k", "v")` | Single query param |
| `:queries({ k = "v" })` | Multiple query params |
| `:body("text")` | Plain text body |
| `:body_bytes("raw")` | Raw bytes body |
| `:json('{"k":"v"}')` | JSON body as raw JSON string |
| `:form({ k = "v" })` | Form body |
| `:timeout_ms(5000)` | Request timeout in milliseconds |
| `:bearer("token")` | Sets `Authorization: Bearer <token>` |

Execute with one of:

`get`, `post`, `put`, `patch`, `delete`, `send`

`send` uses current/default method (`GET` unless set by another verb).

### HTTP Response Shape

Each request returns a response table:

| Field | Type | Description |
|---|---|---|
| `status` | `integer` | HTTP status code |
| `ok` | `boolean` | `true` for `2xx` responses |
| `body` | `string` | Raw response body |
| `url` | `string` | Final response URL |
| `duration_ms` | `integer` | Request duration |
| `headers` | `table<string, string>` | Response headers |
| `json` | `any?` | Parsed JSON (if body is valid JSON) |

## End-to-End Example

```lua
local client = http()
client:base_url("https://jsonplaceholder.typicode.com")

test("GET /posts returns data", function()
  local res = client
    :req()
    :path("/posts")
    :query("_limit", "1")
    :get()

  expect(res.status):to_equal(200)
  expect(res.json):to_exist()
  expect(res.json[1].id):to_exist()
end)

test("POST /posts creates resource", function()
  local res = client
    :req()
    :path("/posts")
    :header("content-type", "application/json")
    :json('{"title":"hello","body":"world","userId":1}')
    :post()

  expect(res.status):to_equal(201)
  expect(res.json.id):to_exist()
end)
```

## Development

Run from source:

```bash
cargo run -- test
```
