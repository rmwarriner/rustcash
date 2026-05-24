# ADR 004 — WASM plugins via `wasmtime` for reports and importers

**Status**: Accepted

## Context

GnuCash uses Guile/Scheme for reports — a significant maintenance burden and a barrier
to community report development. We want third-party reports without native code risks.

## Decision

Plugins are WebAssembly modules loaded by `wasmtime`. They receive a `ReportContext`
(or `ImportContext`) serialized as JSON over a host-defined interface and return output.
Plugins are sandboxed: no filesystem access, no network unless explicitly granted.

## Consequences

- Plugins can be written in any language that compiles to `wasm32-wasi`.
- Safe to install community plugins — no arbitrary native code execution.
- Plugin distribution: a single `.wasm` file + `plugin.toml` manifest.
- Performance overhead vs native is acceptable for report rendering (not a hot path).
- The plugin SDK is a separate crate targeting `wasm32-wasi`.
