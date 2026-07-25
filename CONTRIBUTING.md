# Contributing to SwissArmyNoife

Thanks for helping build the harness-agnostic capability broker (**sak**).

This repository is the OSS Rust workspace. Operator docs for the wider Agentic workspace live
under [`../docs/`](../docs/) when you clone the monorepo layout.

## Prerequisites

- Rust **stable** (1.75+; CI uses current stable) via [rustup](https://rustup.rs/)
- Components: `rustfmt`, `clippy` (`rustup component add rustfmt clippy`)
- Optional: [`cargo-deny`](https://github.com/EmbarkStudios/cargo-deny) (installed automatically by `scripts/ci_check.*` if missing)

## Setup

```bash
cd SwissArmyNoife
cargo build
cargo run -p cli -- hello
cargo run -p cli -- schema dump
```

Config / DB paths: see [`../docs/env.md`](../docs/env.md) (`CONFIG_DIR`, `DB_PATH`, `VAULT_KEY`).

## Development loop

1. Keep changes focused and tested.
2. Run the local gate from this directory:

```powershell
.\scripts\ci_check.ps1
```

```bash
./scripts/ci_check.sh
```

That runs: `fmt --check` → `clippy -D warnings` → `cargo test --workspace` →
`xtask boundaries` → `cargo deny check licenses`.

Optional file-size gate:

```bash
cargo run -p xtask -- size
```

Useful one-offs:

```bash
cargo test -p types
cargo run -p xtask -- boundaries
cargo deny check licenses
```

Local CI command map: [`../docs/ci-matrix.md`](../docs/ci-matrix.md).

## Crate layout (current)

| Crate | Role |
|-------|------|
| `types` | Wire types, error codes, JSON Schema export |
| `control` | `Offer` trait, invoke tracing helpers |
| `env` | path resolution (`CONFIG_DIR`, `DB_PATH`, …) |
| `persist-sqlite` | SQLite migrations / `broker.db` |
| `vault` | AEAD secret encrypt/decrypt |
| `cli` | `sak` binary |
| `mcp` / `mcp-http` | MCP stdio + HTTP surfaces |
| `xtask` | Maintainer tasks (`boundaries`, `size`) |

Do not add Nimbusware dependencies. Do not put marketplace web/billing in this repo.

## Pull requests

- Prefer small, tested PRs.
- Ensure `ci_check` is green.
- Do not commit secrets, `.env`, or machine-local `.cursor/mcp.json`.

## License

Contributions are accepted under **Apache-2.0** (see [LICENSE](LICENSE)).
