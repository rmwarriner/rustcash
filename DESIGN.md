# RustCash — Architecture & Design Document

> A modern, modular personal and small-business accounting platform built in Rust.
> Inspired by GnuCash but not a port — a ground-up rethink of what accounting software
> should look like in 2025.

---

## Vision

GnuCash is reliable but 25+ years of accumulated C/C++/Scheme shows: a monolithic process,
no API, a dated GTK UI, always-on business features, and reports written in Scheme. RustCash
corrects all of this by treating the accounting engine as a library, the HTTP API as the
canonical interface, and every surface (CLI, TUI, GUI) as a thin client over that API.

### Design Goals

- **Layered, dependency-clean crates**: core has no I/O; interfaces have no SQL
- **API-first**: the HTTP API is how everything talks to the engine — GUIs, CLIs, scripts, and third-party tools
- **Three first-class interfaces**: CLI, TUI, and GUI all ship; none is an afterthought
- **WASM-sandboxed plugins**: reports and importers can be written in any language that compiles to WASM
- **Opt-in business features**: invoicing, customers, vendors, payroll live in a separate crate and are not compiled in unless needed
- **Exact arithmetic**: `rust_decimal` everywhere — no floats, ever
- **SQLite-first, PostgreSQL-capable**: single-file databases for personal use; PostgreSQL for teams
- **Modern GUI without GTK pain**: Tauri (web-tech frontend over a Rust backend) or `iced`

---

## Workspace Layout

```
rustcash/
  Cargo.toml            ← workspace root
  DESIGN.md             ← this file
  CLAUDE.md             ← project context for AI-assisted development
  docs/
    adr/                ← architecture decision records
    reports-plugin-api/ ← WASM plugin authoring guide
  crates/
    core/               ← pure domain model, no I/O
    storage/            ← SQLite + PostgreSQL backends
    engine/             ← accounting logic layer
    reports/            ← report trait + standard report library
    import/             ← file format importers
    export/             ← file format exporters
    api/                ← HTTP/JSON API server
    plugin/             ← WASM plugin host
    cli/                ← command-line interface
    tui/                ← terminal user interface
    gui/                ← desktop GUI (Tauri)
    business/           ← opt-in: invoicing, AR/AP, payroll
    sync/               ← opt-in: CRDT multi-device sync
```

### Dependency Rules (enforced by crate boundaries)

```
core       ← no dependencies on other rustcash crates
storage    ← depends on core
engine     ← depends on core, storage
reports    ← depends on core, engine
import     ← depends on core, storage
export     ← depends on core, reports
api        ← depends on engine, reports, import, export, plugin
plugin     ← depends on core (provides WASM host)
cli        ← depends on engine, import, export, reports
tui        ← depends on engine, import, export, reports
gui        ← depends on api (talks to local api server) OR engine directly
business   ← depends on core, engine, storage (opt-in crate)
sync       ← depends on core, storage (opt-in crate)
```

**The golden rule**: nothing below `api` in the stack knows about HTTP. Nothing below `engine`
knows about file I/O. `core` has zero I/O of any kind.

---

## Crate Details

### `core` — Domain Model

The foundation. Pure Rust structs and enums representing the double-entry bookkeeping model.
No async, no I/O, no network, no database. Only dependencies: `rust_decimal`, `chrono`, `uuid`, `serde`.

**Key types:**

```rust
// All IDs are typed newtypes — never pass a raw Uuid where an AccountId is expected
pub struct AccountId(Uuid);
pub struct TransactionId(Uuid);
pub struct CommodityId(Uuid);
pub struct BookId(Uuid);

pub enum AccountType {
    Asset, Liability, Equity, Income, Expense,
    // Sub-types:
    Cash, Bank, CreditCard, Investment, Receivable, Payable, // ...
}

pub struct Account {
    pub id: AccountId,
    pub name: String,
    pub account_type: AccountType,
    pub commodity: CommodityId,
    pub parent: Option<AccountId>,
    pub description: Option<String>,
    pub placeholder: bool,     // container-only, no direct transactions
    pub hidden: bool,
}

pub struct Transaction {
    pub id: TransactionId,
    pub date: NaiveDate,
    pub description: String,
    pub splits: Vec<Split>,     // always sum to zero (validated)
    pub tags: Vec<String>,
    pub notes: Option<String>,
    pub entered_at: DateTime<Utc>,
    pub modified_at: DateTime<Utc>,
}

pub struct Split {
    pub account: AccountId,
    pub amount: Decimal,        // in account's commodity
    pub value: Decimal,         // in transaction's commodity (for multi-currency)
    pub commodity: CommodityId,
    pub reconciled: ReconcileState,
    pub memo: Option<String>,
    pub tags: Vec<String>,      // first-class, per-split (complements transaction-level tags)
}

pub enum ReconcileState { Unreconciled, Cleared, Reconciled }

pub struct Commodity {
    pub id: CommodityId,
    pub namespace: String,      // "CURRENCY", "NYSE", "FUND", etc.
    pub mnemonic: String,       // "USD", "AAPL", etc.
    pub name: String,
    pub fraction: u32,          // smallest unit (100 = cents, 1000 = mils)
}

pub struct Price {
    pub commodity: CommodityId,
    pub currency: CommodityId,
    pub date: NaiveDate,
    pub value: Decimal,
    pub source: PriceSource,    // Manual, AlphaVantage, etc.
}

pub struct Budget {
    pub id: BudgetId,
    pub name: String,
    pub periods: Vec<BudgetPeriod>,
    pub allocations: Vec<BudgetAllocation>,
}
```

**Invariants enforced in core:**
- Transaction splits must sum to zero (validated on construction)
- Account trees have no cycles
- Amounts use `rust_decimal` — floats are never used, never accepted

---

### `storage` — Persistence Layer

SQLite via `sqlx` with compile-time verified queries. PostgreSQL support via the same trait interface.

**Design:**
- `StorageBackend` trait abstracts over SQLite vs PostgreSQL
- Migrations managed by `sqlx migrate` with embedded migration files
- No ORM — plain SQL with typed `sqlx` query macros
- Repository pattern: `AccountRepository`, `TransactionRepository`, etc.
- Transactions (SQL transactions) wrap multi-step operations

**SQLite specifics:**
- WAL mode enabled by default
- Single-writer, multi-reader
- File locking handled by SQLite
- Suitable for personal use (one user, one device or shared via sync)

**PostgreSQL specifics:**
- For team/multi-user deployments
- Connection pooling via `sqlx::PgPool`
- Row-level locking for concurrent transaction entry

**Schema design principles:**
- UUIDs as primary keys (typed in application layer)
- `created_at` / `modified_at` on all rows
- Soft deletes (deleted_at) — financial records should not vanish
- JSON columns for extensible metadata (tags, custom fields)

---

### `engine` — Business Logic

Accounting logic that sits above storage. Stateless functions that take repositories and return results.
No UI, no HTTP, no file I/O.

**Responsibilities:**
- Balance calculations (account balance, running balance, cleared balance)
- Account tree traversal and aggregation
- Reconciliation workflows
- Scheduled transaction generation
- Budget vs actual comparisons
- Lot tracking (for investment cost basis)
- Currency conversion using price history
- Search/filter query builder

**Key interfaces:**

```rust
pub struct AccountBalance {
    pub account: AccountId,
    pub balance: Decimal,
    pub cleared_balance: Decimal,
    pub reconciled_balance: Decimal,
    pub as_of: NaiveDate,
}

pub async fn account_balance(
    repo: &dyn TransactionRepository,
    account: AccountId,
    as_of: NaiveDate,
) -> Result<AccountBalance>;

pub async fn account_tree_balances(
    repo: &dyn AccountRepository,
    txn_repo: &dyn TransactionRepository,
    root: Option<AccountId>,
    as_of: NaiveDate,
) -> Result<Vec<AccountBalance>>;
```

---

### `reports` — Reporting Engine

A trait-based reporting system where reports are data transformations, not presentation.

**Report trait:**

```rust
pub struct ReportMetadata {
    pub id: &'static str,       // "net-worth", "income-statement", etc.
    pub name: &'static str,
    pub description: &'static str,
    pub parameters: Vec<ReportParameter>,
    pub author: Option<&'static str>,
    pub version: &'static str,
}

pub struct ReportContext<'a> {
    pub engine: &'a dyn EngineAccess,  // read-only access to accounting data
    pub params: HashMap<String, ParamValue>,
    pub date_range: DateRange,
    pub commodity: CommodityId,        // reporting currency
}

pub enum ReportOutput {
    Html(String),
    Pdf(Vec<u8>),
    Csv(String),
    Json(serde_json::Value),
}

pub trait Report: Send + Sync {
    fn metadata(&self) -> &ReportMetadata;
    fn render(&self, ctx: &ReportContext) -> Result<ReportOutput>;
}
```

**Standard reports shipped with RustCash:**
- Income Statement (Profit & Loss)
- Balance Sheet
- Net Worth over time
- Cash Flow Statement
- Account Transaction List
- Budget vs Actual
- Investment Portfolio
- Tax Summary
- Reconciliation Report

**Third-party reports**: loaded as WASM modules via the `plugin` crate. The WASM module receives
a `ReportContext` serialized as JSON and returns a `ReportOutput`. No unsafe access to the host.

---

### `import` — File Importers and Live Data Downloaders

The `import` crate handles two distinct data-ingestion paths that share the same
review-and-confirm flow:

**File importers** parse a local file into an `ImportPreview`. The user reviews the
preview and confirms before anything is written to storage.

**Live downloaders** fetch transactions from a remote financial data service
(e.g. SimpleFIN Bridge, Plaid, OFX Direct Connect) and produce the same `ImportPreview`
for review. The user still confirms before anything is stored — the only difference is
the data source.

Both paths funnel through `ImportPreview` so the confirmation UI, duplicate detection,
and storage commit logic are shared.

```rust
pub trait Importer: Send + Sync {
    fn name(&self) -> &str;
    fn supported_extensions(&self) -> &[&str];
    fn import(&self, source: &mut dyn Read, book: BookId) -> Result<ImportPreview>;
}

/// A live financial data source (bank feed, aggregator, etc.).
/// Async because it performs network I/O.
pub trait Downloader: Send + Sync {
    fn name(&self) -> &str;

    /// Fetch transactions since `since` (or all available if None).
    async fn fetch(
        &self,
        credentials: &DownloaderCredentials,
        book_id: BookId,
        since: Option<NaiveDate>,
    ) -> Result<ImportPreview, ImportError>;
}

/// Opaque credential bundle stored by the caller (storage crate or API layer).
/// The `Downloader` impl interprets the fields; `import` does not.
pub struct DownloaderCredentials {
    pub fields: std::collections::HashMap<String, String>,
}

pub struct ImportPreview {
    pub transactions: Vec<Transaction>,
    pub new_accounts: Vec<Account>,
    pub duplicates: Vec<DuplicateCandidate>,  // matched against existing transactions
    pub warnings: Vec<String>,
}
```

**Credential storage**: credentials are persisted by the `storage` crate (encrypted at
rest). The `import` crate defines the `DownloaderCredentials` shape but has no knowledge
of how they are stored or encrypted.

**Planned live downloaders:**
- `SimplefinDownloader` — SimpleFIN Bridge (open protocol, self-hostable)
- `OfxDirectDownloader` — OFX Direct Connect (bank-hosted OFX server)

**Planned file importers:**
- `CsvImporter` — configurable column mapping
- `OfxImporter` — OFX/QFX (bank/broker exports)
- `QifImporter` — QIF (Quicken)
- `GnuCashXmlImporter` — migrate from GnuCash
- `GnuCashSqlImporter` — migrate from GnuCash SQLite

---

### `export` — File Exporters

Mirror of import. Exporters implement `Exporter` and produce files. Standard formats:
- CSV (transactions, account list)
- OFX (for sharing with other apps)
- GnuCash XML (round-trip compatibility during migration)

---

### `api` — HTTP/JSON API

Built with `axum`. This is the canonical interface — everything talks through here.

**Design:**
- REST + JSON
- OpenAPI spec auto-generated via `utoipa`
- Auth: API tokens (Bearer) for programmatic access; session cookies for the GUI
- All endpoints are async
- Versioned: `/v1/...`
- Pagination on list endpoints (cursor-based)

**Key endpoint groups:**

```
GET    /v1/books                    — list books (databases)
POST   /v1/books                    — create book

GET    /v1/accounts                 — list accounts (tree or flat)
POST   /v1/accounts                 — create account
GET    /v1/accounts/:id             — get account
PATCH  /v1/accounts/:id             — update account
GET    /v1/accounts/:id/balance     — account balance (as-of date)
GET    /v1/accounts/:id/transactions — transactions for account

GET    /v1/transactions             — list/search transactions
POST   /v1/transactions             — create transaction
GET    /v1/transactions/:id         — get transaction
PUT    /v1/transactions/:id         — replace transaction
DELETE /v1/transactions/:id         — soft delete

GET    /v1/commodities              — list commodities/currencies
GET    /v1/prices                   — price history
POST   /v1/prices                   — add price

GET    /v1/reports                  — list available reports
POST   /v1/reports/:id/render       — render report (params in body)

POST   /v1/import                   — upload file, returns ImportPreview
POST   /v1/import/confirm           — confirm import after preview

GET    /v1/plugins                  — list installed WASM plugins
POST   /v1/plugins                  — install plugin (upload WASM)
DELETE /v1/plugins/:id              — remove plugin

# Business (only available if business crate is compiled in)
GET    /v1/customers
GET    /v1/invoices
POST   /v1/invoices/:id/post        — post invoice (creates transaction)
```

---

### `plugin` — WASM Plugin Host

Uses `wasmtime` to load and execute WASM modules in a sandboxed environment.

**What plugins can do:**
- Implement the `Report` interface
- Implement the `Importer` interface
- Implement the `Exporter` interface
- Add custom account types (metadata only)

**What plugins cannot do:**
- Access the filesystem directly
- Make network requests (unless explicitly granted)
- Access host memory outside their sandbox

**Plugin manifest (plugin.toml):**
```toml
[plugin]
id = "my-cashflow-report"
name = "Cash Flow Waterfall"
version = "1.0.0"
author = "Jane Developer"
type = "report"           # "report" | "importer" | "exporter"
wasm = "cashflow.wasm"
```

**Plugin distribution**: plugins are single `.wasm` files plus a `plugin.toml`. No native code,
no install scripts — safe to install from the community.

---

### `cli` — Command-Line Interface

Built with `clap`. First-class interface, not a debugging tool.

**Design goals:**
- Machine-readable output (`--format json`)
- Scriptable: pipe-friendly, exit codes, no interactive prompts unless opted in
- Composable with standard Unix tools

**Command structure:**

```
rustcash account list [--format table|json|csv]
rustcash account show <id>
rustcash account balance <id> [--as-of YYYY-MM-DD]
rustcash account tree

rustcash transaction list [--account <id>] [--from DATE] [--to DATE]
rustcash transaction add    ← interactive or --from-json
rustcash transaction show <id>
rustcash transaction delete <id>

rustcash import <file> [--format csv|ofx|qif|gnucash]
rustcash export <file> [--format csv|ofx]

rustcash report list
rustcash report render <report-id> [--from DATE] [--to DATE] [--format html|csv|json]

rustcash reconcile <account-id> --statement-date DATE --statement-balance AMOUNT

rustcash serve [--port 8080]    ← start the API server

rustcash plugin list
rustcash plugin install <path.wasm>
rustcash plugin remove <id>
```

---

### `tui` — Terminal User Interface

Built with `ratatui`. Targets power users who live in the terminal.

**Layout:**
```
┌─ Accounts ──────────┬─ Transactions ──────────────────────────────┐
│ > Assets            │ Date       Description          Amount  Bal  │
│   > Checking  1,234 │ 2025-05-01 Salary              3,500   ...  │
│   > Savings   8,900 │ 2025-05-03 Grocery Store         -87   ...  │
│ > Liabilities       │ 2025-05-05 Electric Bill        -120   ...  │
│   > Visa     -2,100 │ ...                                         │
│ > Income            │                                             │
│ > Expenses          │                                             │
└─────────────────────┴─────────────────────────────────────────────┘
│ [n]ew  [e]dit  [d]elete  [r]econcile  [/]search  [R]eport  [q]uit│
└─────────────────────────────────────────────────────────────────────┘
```

**Features:**
- Vim-style keybindings (hjkl navigation)
- Transaction entry with account autocomplete
- Reconciliation workflow
- Inline report preview (rendered as text/table)
- Fuzzy search across all transactions

---

### `gui` — Desktop GUI

**Technology choice: Tauri**

Tauri embeds a Rust backend (the `api` crate running locally) with a webview frontend.
This gives:
- Native OS window chrome and system tray
- Any web UI framework for the frontend (React, Svelte, Vue — TBD)
- Access to native file dialogs, notifications, OS keychain
- Reports rendered natively in the webview (HTML reports look great)
- Smaller binary than Electron

**Alternative: `iced`**
If a pure-Rust GUI is preferred, `iced` is the strongest option. Tradeoff: harder to style,
no HTML report rendering, but no webview dependency.

**GUI feature targets:**
- Account tree with drag-to-reorder
- Spreadsheet-style transaction register (keyboard-driven entry)
- Report viewer with parameter forms
- Import wizard (drag-and-drop file, column mapping UI)
- Dashboard with configurable widgets (net worth chart, budget gauges, recent transactions)
- Plugin manager UI
- Preferences (themes, default commodities, date format, etc.)

---

### `business` — Optional Business Features

Compiled in only when needed. Adds:

```rust
// New domain types (extends core via separate module)
pub struct Customer { ... }
pub struct Vendor { ... }
pub struct Employee { ... }
pub struct Invoice { ... }      // AR
pub struct Bill { ... }         // AP
pub struct PayrollEntry { ... }

// New API endpoints registered when business crate is present
// New reports: A/R Aging, A/P Aging, Profit & Loss by Customer
// New account types: Receivable, Payable
```

Activated via Cargo feature flag:
```toml
[features]
default = []
business = ["dep:rustcash-business"]
```

---

### `sync` — Optional Multi-Device Sync

CRDT-based sync so two devices can work offline and merge cleanly.

- Uses `automerge` or a custom CRDT tailored to the transaction model
- Sync backends: local network (mDNS discovery), or a simple relay server
- Conflict resolution: transactions are immutable once posted; edits create new versions
- No cloud vendor lock-in — bring your own relay

---

## Technology Stack

| Concern | Choice | Rationale |
|---|---|---|
| Language | Rust (2024 edition) | Memory safety, performance, ecosystem |
| Arithmetic | `rust_decimal` | Exact decimal math, no float rounding errors |
| Dates | `chrono` | De facto standard, good timezone support |
| IDs | `uuid` v4 | Random, no coordination needed |
| Serialization | `serde` + `serde_json` | Universal |
| Async runtime | `tokio` | Standard for async Rust |
| Database (primary) | SQLite via `sqlx` | Zero-config, file-based, battle-tested |
| Database (team) | PostgreSQL via `sqlx` | Multi-user, same trait interface |
| HTTP framework | `axum` | Ergonomic, tower-compatible, well-maintained |
| OpenAPI | `utoipa` | Derive-based, low ceremony |
| CLI | `clap` | Best-in-class, derive API |
| TUI | `ratatui` | Actively maintained, feature-rich |
| Desktop GUI | Tauri | Web tech UI + Rust backend |
| WASM host | `wasmtime` | Production-grade WASM runtime |
| Error handling | `thiserror` (libs) + `anyhow` (bins) | Standard pattern |
| Logging | `tracing` + `tracing-subscriber` | Structured, async-aware |
| Testing | `cargo test` + `proptest` | Property testing for accounting invariants |
| Config | `config` crate + TOML | Layered config (defaults → file → env → flags) |

---

## Data Model — SQL Schema Sketch

```sql
-- Books (one per file/database, but schema is per-book for PostgreSQL multi-tenant)
CREATE TABLE books (
    id          UUID PRIMARY KEY,
    name        TEXT NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    modified_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE commodities (
    id          UUID PRIMARY KEY,
    book_id     UUID NOT NULL REFERENCES books(id),
    namespace   TEXT NOT NULL,   -- 'CURRENCY', 'NYSE', 'FUND'
    mnemonic    TEXT NOT NULL,   -- 'USD', 'AAPL'
    name        TEXT NOT NULL,
    fraction    INTEGER NOT NULL DEFAULT 100,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE accounts (
    id              UUID PRIMARY KEY,
    book_id         UUID NOT NULL REFERENCES books(id),
    parent_id       UUID REFERENCES accounts(id),
    name            TEXT NOT NULL,
    account_type    TEXT NOT NULL,
    commodity_id    UUID NOT NULL REFERENCES commodities(id),
    description     TEXT,
    placeholder     BOOLEAN NOT NULL DEFAULT false,
    hidden          BOOLEAN NOT NULL DEFAULT false,
    sort_order      INTEGER NOT NULL DEFAULT 0,
    metadata        JSONB,       -- extensible key-value
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    modified_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at      TIMESTAMPTZ  -- soft delete
);

CREATE TABLE transactions (
    id              UUID PRIMARY KEY,
    book_id         UUID NOT NULL REFERENCES books(id),
    date            DATE NOT NULL,
    description     TEXT NOT NULL DEFAULT '',
    notes           TEXT,
    tags            TEXT[] NOT NULL DEFAULT '{}',
    entered_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    modified_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at      TIMESTAMPTZ
);

CREATE TABLE splits (
    id              UUID PRIMARY KEY,
    transaction_id  UUID NOT NULL REFERENCES transactions(id),
    account_id      UUID NOT NULL REFERENCES accounts(id),
    amount          NUMERIC(20,8) NOT NULL,  -- in account's commodity
    value           NUMERIC(20,8) NOT NULL,  -- in transaction commodity
    commodity_id    UUID NOT NULL REFERENCES commodities(id),
    reconcile_state TEXT NOT NULL DEFAULT 'n',  -- 'n', 'c', 'y'
    reconcile_date  DATE,
    memo            TEXT,
    tags            TEXT NOT NULL DEFAULT '[]',  -- JSON array; first-class per-split tags
    action          TEXT,  -- 'Buy', 'Sell', 'Div', etc. for investments
    lot_id          UUID,  -- for cost-basis lot tracking
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE prices (
    id              UUID PRIMARY KEY,
    book_id         UUID NOT NULL REFERENCES books(id),
    commodity_id    UUID NOT NULL REFERENCES commodities(id),
    currency_id     UUID NOT NULL REFERENCES commodities(id),
    date            DATE NOT NULL,
    value           NUMERIC(20,8) NOT NULL,
    source          TEXT NOT NULL DEFAULT 'user',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Indexes
CREATE INDEX idx_splits_account_id ON splits(account_id);
CREATE INDEX idx_splits_transaction_id ON splits(transaction_id);
CREATE INDEX idx_transactions_date ON transactions(date);
CREATE INDEX idx_transactions_book_date ON transactions(book_id, date);
CREATE INDEX idx_accounts_parent ON accounts(parent_id);
CREATE INDEX idx_prices_commodity_date ON prices(commodity_id, date);
```

---

## Configuration

RustCash uses layered configuration (later layers override earlier):

1. Built-in defaults
2. System config: `/etc/rustcash/config.toml`
3. User config: `~/.config/rustcash/config.toml`
4. Book-level config: stored in the database
5. Environment variables: `RUSTCASH_*`
6. CLI flags

```toml
# ~/.config/rustcash/config.toml

[database]
url = "~/.local/share/rustcash/default.db"  # SQLite path or postgres:// URL
pool_size = 5

[api]
bind = "127.0.0.1:8080"
token = ""   # leave blank to disable auth for local-only use

[reporting]
default_format = "html"
plugin_dir = "~/.local/share/rustcash/plugins"

[ui]
date_format = "%Y-%m-%d"
default_commodity = "USD"
theme = "system"   # "light" | "dark" | "system"
```

---

## Plugin Development Guide (Summary)

Third-party developers write plugins in any language that compiles to WASM (Rust, Go, AssemblyScript, C, etc.).

**Minimal Rust report plugin:**

```rust
// In plugin's lib.rs — compiled to wasm32-wasi target
use rustcash_plugin_sdk::prelude::*;

#[report]
pub struct MyReport;

impl Report for MyReport {
    fn metadata(&self) -> &ReportMetadata {
        static META: ReportMetadata = ReportMetadata {
            id: "my-report",
            name: "My Custom Report",
            description: "Does something useful",
            parameters: &[],
            author: Some("Jane Dev"),
            version: "1.0.0",
        };
        &META
    }

    fn render(&self, ctx: &ReportContext) -> Result<ReportOutput> {
        let accounts = ctx.engine.accounts()?;
        // ... build HTML string
        Ok(ReportOutput::Html(html))
    }
}
```

Compile with: `cargo build --target wasm32-wasi --release`
Install with: `rustcash plugin install target/wasm32-wasi/release/my_report.wasm`

---

## Migration from GnuCash

RustCash ships a first-class migration path:

```bash
rustcash import ~/Documents/my-finances.gnucash --format gnucash-xml
# or
rustcash import --format gnucash-sqlite --db ~/snap/gnucash/current/.local/share/gnucash/...
```

The GnuCash importer handles:
- All account types and hierarchy
- All transactions and splits (with reconciliation state)
- Commodity/currency definitions
- Price history
- Scheduled transactions → converted to RustCash scheduled transaction model
- Business objects (customers, invoices) → requires `business` feature

What does not migrate:
- Guile/Scheme reports → must be rewritten as WASM plugins
- GnuCash-specific UI preferences

---

## Phased Development Plan

### Phase 1 — Foundation (Milestone: `cargo test` passes, real data in SQLite)
- [ ] Workspace scaffold with all crate stubs
- [ ] `core`: all domain types, invariant validation
- [ ] `storage`: SQLite backend, migrations, all repositories
- [ ] `engine`: balance calculations, account tree aggregation
- [ ] `import/csv`: basic CSV importer for real-world testing
- [ ] `import/gnucash-xml`: GnuCash migration importer
- [ ] Property-based tests for accounting invariants (splits sum to zero, balance consistency)

### Phase 2 — CLI & API (Milestone: fully usable from terminal)
- [ ] `cli`: account/transaction CRUD, import, basic balance report
- [ ] `api`: all REST endpoints, OpenAPI spec generated
- [ ] `reports`: report trait, Income Statement, Balance Sheet, Transaction List
- [ ] `import/ofx`: OFX/QFX import (bank statement download)

### Phase 3 — TUI (Milestone: GnuCash-replaceable for terminal users)
- [ ] `tui`: account tree, transaction register, reconciliation workflow
- [ ] `tui`: report viewer (text/table output)
- [ ] `reports`: Net Worth, Cash Flow, Budget vs Actual

### Phase 4 — GUI (Milestone: modern desktop app)
- [ ] Tauri app scaffold
- [ ] Account tree, transaction register, report viewer
- [ ] Import wizard UI, plugin manager UI
- [ ] Dashboard with configurable widgets

### Phase 5 — Plugins & Ecosystem
- [ ] `plugin`: WASM host, plugin manifest, install/remove
- [ ] Plugin SDK crate with WASM-compatible types
- [ ] Example report plugin (published to crates.io)
- [ ] Plugin registry (GitHub-based initially)

### Phase 6 — Business & Sync (Opt-in)
- [ ] `business`: customers, vendors, invoices, bills, A/R, A/P
- [ ] `sync`: CRDT engine, local network sync, relay server

---

## Architecture Decision Records

ADRs live in `docs/adr/`. Current decisions:

| # | Decision | Rationale |
|---|---|---|
| 001 | Use `rust_decimal` not f64 | Financial arithmetic requires exact decimal representation |
| 002 | SQLite as primary, PostgreSQL optional | Zero-config for personal use; same trait for team use |
| 003 | API-first: all interfaces talk to `engine` or `api` | Prevents UI coupling to storage; enables third-party clients |
| 004 | WASM plugins via `wasmtime` | Language-agnostic, safe sandbox, no native code required |
| 005 | Tauri for GUI over GTK/iced | Web-tech UI enables rich report rendering; smaller than Electron |
| 006 | Soft deletes for financial records | Financial records must not disappear; audit trail required |
| 007 | Typed ID newtypes over raw Uuid | Prevents passing AccountId where TransactionId expected |
| 008 | `business` as opt-in crate | Personal finance users pay no complexity cost for business features |
| 009 | `thiserror` in libs, `anyhow` in bins | Clean error types in library API; flexible handling in binaries |
| 010 | Cursor-based pagination on list endpoints | Stable under concurrent inserts; works with large datasets |
