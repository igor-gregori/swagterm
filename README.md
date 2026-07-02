# SwagTerm

A terminal Swagger/OpenAPI viewer. Browse, inspect, and test API endpoints directly from your terminal — close to the Swagger UI web experience.

## Install

Requires [Rust](https://rustup.rs/) installed.

```bash
cargo install --git https://github.com/igor-gregori/swagterm.git
```

## Usage

```bash
# Local file
swagterm petstore.json
swagterm api-spec.yaml

# From URL
swagterm https://petstore.swagger.io/v2/swagger.json

# With auth
swagterm petstore.json --bearer "your-token"
swagterm petstore.json --api-key "X-API-Key=abc123"
swagterm petstore.json --basic "user:pass"
swagterm petstore.json -H "X-Custom=value"
```

## Keybindings

| Key | Action |
|-----|--------|
| `j`/`k` or `↑`/`↓` | Navigate / Scroll |
| `Tab` | Switch sidebar ↔ detail |
| `Enter`/`Space` | Toggle tag collapse |
| `/` | Search endpoints |
| `t` | Try it out (execute requests) |
| `a` | Configure auth |
| `w` | Toggle spec warnings |
| `q` | Quit |

### Try it out mode

| Key | Action |
|-----|--------|
| `j`/`k` | Select field / Scroll response |
| `Enter` | Edit field value |
| `s` | Send request |
| `c` | Copy as curl |
| `Esc` | Back to browse |

## Features

- Swagger 2.0 and OpenAPI 3.0/3.1 support
- JSON and YAML files
- Load from file or URL
- Endpoints grouped by tag (collapsible)
- Color-coded HTTP methods
- Parameters table with enum values
- Response examples (generated from schema)
- Execute requests with "Try it out"
- Loading spinner for requests
- Copy as curl to clipboard
- Auth configuration (Bearer, API Key, Basic, Custom headers)
- Spec validation warnings
