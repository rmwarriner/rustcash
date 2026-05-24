# ADR 003 — API-first: all interfaces are clients of the HTTP API

**Status**: Accepted

## Context

GnuCash has no HTTP API. Its GUI, reports, and data are tightly coupled to a single process.
Third-party integration requires forking the process or screen-scraping.

## Decision

The `api` crate (axum) is the canonical interface. The CLI and TUI may call `engine` directly
for performance-sensitive local workflows, but the GUI (Tauri) always talks to the local API
server. External tools, scripts, and future mobile/web clients use the same API.

## Consequences

- The API must be complete and well-documented (OpenAPI spec auto-generated).
- The local server binds to 127.0.0.1 only by default; auth tokens are optional for
  single-user local use.
- GUI development is decoupled from Rust — any web framework can be used.
- API versioning (`/v1/`) must be maintained from the start.
