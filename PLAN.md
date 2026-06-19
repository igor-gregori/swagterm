# SwagTerm — Terminal Swagger Viewer

A TUI application that renders Swagger/OpenAPI specs interactively in the terminal, close to the Swagger Web UI experience.

## Decisions

| Decision | Choice |
|----------|--------|
| Language | Rust |
| TUI framework | ratatui |
| Swagger version | 2.0 (MVP) |
| Input source | Local file only (MVP) |
| Mode | Read-only viewer (MVP) |
| File formats | JSON, YAML/YML |

## Architecture

```
┌─────────────────────────────────────────────────┐
│  CLI Entry Point                                │
│  swagterm <file.json|file.yaml>                 │
├─────────────────────────────────────────────────┤
│  Parser Layer                                   │
│  - Reads JSON/YAML                              │
│  - Deserializes into Swagger 2.0 model          │
│  - Resolves $ref references                     │
├─────────────────────────────────────────────────┤
│  Domain Model                                   │
│  - API Info (title, version, description)       │
│  - Paths → Operations (GET, POST, etc.)         │
│  - Parameters, Request Body, Responses          │
│  - Schemas / Definitions                        │
├─────────────────────────────────────────────────┤
│  TUI Layer (ratatui)                            │
│  - Sidebar: endpoints grouped by tag            │
│  - Main panel: operation detail view            │
│  - Schema viewer (expand/collapse)              │
│  - Search/filter bar                            │
└─────────────────────────────────────────────────┘
```

## MVP Features

- Parse Swagger 2.0 specs from local JSON/YAML files
- Sidebar with endpoints grouped by tag
- Color-coded HTTP methods (GET=green, POST=blue, PUT=yellow, DELETE=red)
- Operation detail view: method, path, summary, description
- Parameters table (name, in, type, required)
- Responses section with status codes and descriptions
- Schema/definition viewer with expand/collapse for nested objects
- Search/filter endpoints by path or keyword
- Keyboard navigation

## Layout

```
┌──────────────────────────────────────────────────────┐
│  SwagTerm — Petstore API v1.0.7                      │
├───────────────┬──────────────────────────────────────┤
│  Tags/Groups  │  GET /pet/{petId}                    │
│               │  Summary: Find pet by ID             │
│ ▼ pet         │                                      │
│   POST /pet   │  Parameters:                         │
│   PUT /pet    │  ┌──────────┬──────┬─────┬────────┐ │
│   GET /pet/fi │  │ Name     │ In   │ Type│Required│ │
│   ...         │  ├──────────┼──────┼─────┼────────┤ │
│               │  │ petId    │ path │ int │ yes    │ │
│ ▼ store       │  └──────────┴──────┴─────┴────────┘ │
│   GET /store  │                                      │
│   POST /store │  Responses:                          │
│               │  200 - successful operation          │
│ ▼ user        │  400 - Invalid ID supplied           │
│   POST /user  │  404 - Pet not found                 │
│   ...         │                                      │
└───────────────┴──────────────────────────────────────┘
```

## Tech Stack

| Crate | Purpose |
|-------|---------|
| ratatui | TUI framework |
| crossterm | Terminal backend |
| serde / serde_json / serde_yaml | Deserialization |
| clap | CLI argument parsing |

## v2 Roadmap

- OpenAPI 3.0 / 3.1 support
- Load spec from URL
- "Try it out" — build and execute requests
- Auth configuration (API key, Bearer, OAuth)
- Copy as curl to clipboard
- Multiple specs (tabs)
- Spec validation warnings
