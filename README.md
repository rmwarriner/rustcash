# RustCash

A modern, modular accounting platform in Rust — inspired by GnuCash, built for the next 25 years.

> **Status**: Phase 1 foundation complete. Domain model, all storage repositories (SQLite), and core engine services (balances, transaction lifecycle, account management) are implemented and fully tested. Next: CSV importer and working CLI/API commands.

---

## Why

GnuCash is reliable software, but 25+ years of accumulated C/C++/Scheme shows: a monolithic process with no API, a GTK UI that shows its age, reports written in Scheme, and business features compiled in whether you want them or not.

RustCash is not a port — it's a ground-up rethink of what personal and small-business accounting software should look like today:

- **Three first-class interfaces** — CLI, TUI, and desktop GUI, all over the same engine
- **HTTP API others can build on** — script it, integrate it, build your own client
- **WASM plugin system** — write reports and importers in any language; install them safely
- **Modern GUI** — Tauri-based, not GTK
- **Business features are opt-in** — personal finance users pay zero cost for invoicing and payroll
- **Exact arithmetic** — `rust_decimal` throughout; floats are never used for money

---

## Architecture

The project is a Cargo workspace of focused crates. Nothing flows upward — `core` has no I/O, `engine` has no HTTP, the GUI has no SQL.

```
crates/
  core/       Pure domain model: Account, Transaction, Split, Commodity, Price, Budget
  storage/    SQLite (default) and PostgreSQL repositories via sqlx
  engine/     Accounting logic: balances, reconciliation, budgeting, scheduling
  reports/    Report trait + standard report library (income statement, balance sheet, …)
  import/     File importers: CSV, OFX/QFX, QIF, GnuCash XML
  export/     File exporters: CSV, OFX
  api/        axum HTTP/JSON API — the canonical interface for all clients
  plugin/     wasmtime WASM host for sandboxed third-party reports and importers
  cli/        clap CLI — scriptable, pipe-friendly, JSON output
  tui/        ratatui TUI — vim keybindings, account tree, transaction register
  gui/        Tauri desktop app
  business/   Opt-in: invoicing, AR/AP, customers, vendors, payroll
  sync/       Opt-in: CRDT-based multi-device sync
```

See [`DESIGN.md`](DESIGN.md) for the full architecture, data model, API design, plugin system, SQL schema, and phased development plan.

---

## Key Design Decisions

| Decision | Choice | Why |
|---|---|---|
| Money arithmetic | `rust_decimal` | Exact decimal — floats are not acceptable for financial data |
| Primary database | SQLite via `sqlx` | Zero-config file-based database; PostgreSQL available for teams |
| Interface model | API-first | All UIs are clients; CLI and TUI also access `engine` directly (no server needed) |
| Plugin sandbox | `wasmtime` WASM | Any language, no native code, safe to install from the community |
| Desktop GUI | Tauri | Web-tech UI + Rust backend; HTML reports render natively |
| Business features | Separate opt-in crate | Personal users compile none of it |
| Financial records | Soft deletes only | `deleted_at` — records never vanish |
| ID types | Typed newtypes | `AccountId(Uuid)` — mixing ID types is a compile error |

Architecture decision records live in [`docs/adr/`](docs/adr/).

---

## Getting Started

### Prerequisites

- Rust (stable, 2024 edition) — install via [rustup](https://rustup.rs)

### Build

```bash
git clone https://github.com/rmwarriner/rustcash
cd rustcash
cargo build
```

### Check the whole workspace

```bash
cargo check --workspace
```

### Run the CLI (placeholder)

```bash
cargo run -p rustcash-cli -- --help
```

The CLI accesses the `engine` and storage layers directly — no API server is required for single-user local use.

### Run the API server (placeholder)

```bash
cargo run -p rustcash-api
# listening on http://127.0.0.1:8080
curl http://127.0.0.1:8080/v1/health
```

The API server is only needed when you want remote access, multiple users, or third-party integrations.

---

## Roadmap

### Phase 1 — Foundation *(complete)*
- [x] Workspace scaffold and domain model (`core`)
- [x] SQL migration schema (`storage`)
- [x] Storage repositories: Book, Commodity, Account, Transaction, Price (52 tests)
- [x] Engine services: `BalanceService`, `TransactionService`, `AccountService` (24 tests)
- [x] Report, Importer, and Exporter traits defined
- [x] axum API skeleton with `/v1/health`
- [x] Full clap CLI command tree
- [x] ADRs 001–012 covering all major architectural decisions
- [ ] CSV importer
- [ ] GnuCash XML importer (migration path)

### Phase 2 — CLI & API *(next)*
- [ ] CSV importer (real-world data in SQLite)
- [ ] Working account and transaction CRUD via CLI
- [ ] Complete REST API with OpenAPI spec
- [ ] Income Statement and Balance Sheet reports
- [ ] OFX/QFX importer

### Phase 3 — TUI
- [ ] Account tree and transaction register
- [ ] Reconciliation workflow
- [ ] Report viewer

### Phase 4 — GUI
- [ ] Tauri app with web frontend
- [ ] Import wizard, plugin manager, dashboard

### Phase 5 — Plugins
- [ ] WASM plugin host fully wired
- [ ] Plugin SDK crate (target: `wasm32-wasi`)
- [ ] Example report plugin

### Phase 6 — Business & Sync *(opt-in)*
- [ ] Invoicing, AR/AP, customers, vendors
- [ ] CRDT multi-device sync

---

## Migrating from GnuCash

A first-class migration path is planned:

```bash
rustcash import ~/Documents/my-finances.gnucash --format gnucash-xml
```

The importer will handle accounts, transactions, commodities, price history, and scheduled transactions. Scheme-based reports will need to be rewritten as WASM plugins.

---

## Contributing

The project is in early architecture-and-scaffold stage. The best place to start is the [DESIGN.md](DESIGN.md) to understand the intended shape, then the Phase 1 items above.

Issues and discussion are welcome.

---

## License

AGPL-3.0-or-later. The GNU Affero GPL was chosen over GPL because it closes the "network use" loophole: anyone deploying RustCash as a service must share their modifications.
