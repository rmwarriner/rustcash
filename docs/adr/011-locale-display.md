# ADR 011 — Locale and display formatting

**Status**: Accepted

## Context

`rust_decimal` makes arithmetic locale-agnostic. The threat is in display: if formatting
code starts hardcoding `","` as the thousands separator, `"."` as the decimal separator,
or `"$"` as the currency prefix, those assumptions propagate through the codebase and
become expensive to extract later. This ADR locks in the constraint before the first
display function is written.

## Decisions

### 1. Internal representation is always locale-agnostic

| Data | Type | Serialisation |
|------|------|---------------|
| Monetary amounts | `rust_decimal::Decimal` | Decimal string (`"1234.56"`) |
| Timestamps | `chrono::DateTime<Utc>` | RFC 3339 |
| Accounting dates | `chrono::NaiveDate` | ISO 8601 (`YYYY-MM-DD`) |

None of these types carry locale information. Locale is applied only at the display layer.

### 2. All user-facing display is locale-parameterized — never hardcoded

Any function that produces a user-visible string containing a date, number, or currency
amount must accept a formatting config (locale, format string, or equivalent). It must
not call `format!("{}", amount)` or `amount.to_string()` directly for user output.

The specific type (`DisplayConfig`, `Locale`, format string) is defined when the first
display function is written, in the crate that needs it. It is **not** in `core` — core
has zero display logic. It lives in `api` (for JSON response formatting), `cli` (for
terminal output), `tui` (for rendered cells), or `reports` (for report output).

### 3. Currency display is commodity-driven, not locale-driven

Symbol placement (prefix vs. suffix), spacing, and the number of decimal places are
determined by the `Commodity` record:
- `fraction` (e.g., 100 for two decimal places, 1000 for three) controls precision.
- Symbol and placement are stored on `Commodity` when that data is added to the schema.

Locale affects only the decimal separator and grouping separator (e.g., `.` vs `,`).
The currency symbol and precision come from the commodity record.

### 4. API responses use locale-agnostic formats

The HTTP API always serialises amounts as decimal strings and dates as ISO 8601. The API
does not apply locale formatting. Localisation is the responsibility of the client (GUI,
TUI, CLI) based on the end user's locale preference.

### 5. No translation library for UI strings — yet

All user-facing text (labels, error messages, prompts) is English. When non-English
support becomes a requirement, a translation library (e.g., Fluent) will be added. This
ADR does not block that addition — the constraint here is only about number/date/currency
formatting, not UI string translation.

### 6. The constraint is a code-review gate

A PR that introduces `format!("{}", amount)` or `format!("{}", date)` for user-facing
output will be rejected at review, regardless of how small the change looks. The
enforcement mechanism is code review, not a lint rule (though a custom Clippy lint could
be added later).

## What this does NOT decide

- Which locale library to use (decided when the first display function is written)
- How the user's locale preference is stored or communicated to the server
- Whether the TUI or GUI auto-detects the system locale

## Consequences

- No locale debt accumulates before the first UI is built
- Adding a new locale later requires only new formatting logic, not grep-and-replace
  through hardcoded separators
- The API is clean for non-English clients from day one
- Core and engine remain free of any display or formatting concern
