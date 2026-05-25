# ADR 010 — Observability policy: logging, correlation, and error responses

**Status**: Accepted

## Context

`tracing` is already in the dependency stack. Without a policy, each developer makes
independent choices about log levels, field names, and error response shapes. By the time
the API has a dozen endpoints those choices are load-bearing and hard to change. Deciding
now costs nothing.

## Decisions

### 1. Log format: configurable, defaulting to ANSI

Two formats are supported, selected by `log_format` in `ApiConfig`:

| Value | Output | Use case |
|-------|--------|----------|
| `ansi` (default) | Coloured, human-readable | Terminal development |
| `json` | Structured JSON, one object per line | Production; log aggregators (Loki, Datadog) |

The format can also be overridden by the `RUST_LOG_FORMAT=json|ansi` environment variable,
which takes precedence over the config file value. This lets ops teams configure format
without rebuilding or editing config files.

### 2. Structured log fields

Every log record must include, at minimum:

- `timestamp` — RFC 3339
- `level` — ERROR / WARN / INFO / DEBUG
- `target` — the Rust module path (provided automatically by `tracing`)
- `message`

For records emitted within an API request span, additionally:

- `request_id` — the correlation ID (see §3)
- `method` — HTTP method
- `path` — request path (not the full URL; no query strings in the default format)
- `status` — HTTP status code (on the response record)
- `duration_ms` — wall-clock time for the full request (on the response record)

### 3. Correlation IDs via X-Request-ID

Every API request gets a `request_id`:
- If the client sends `X-Request-ID: <value>`, that value is used.
- Otherwise, a new UUIDv4 is generated.
- The `request_id` is echoed back in the response as `X-Request-ID`.
- All `tracing` records emitted during the request are inside a span that carries
  `request_id` as a field, so log aggregators can join them.

### 4. API error envelope: RFC 7807 Problem Details

All API error responses use `Content-Type: application/problem+json` with the RFC 7807
shape:

```json
{
  "type": "https://rustcash.app/errors/not-found",
  "title": "Not Found",
  "status": 404,
  "detail": "Account 3fa85f64-5717-4562-b3fc-2c963f66afa6 does not exist in this book.",
  "instance": "/v1/books/abc/accounts/3fa85f64-5717-4562-b3fc-2c963f66afa6"
}
```

- `type` is a URI that identifies the error class. It may be a documentation URL or a
  URN. It must be stable — clients may key on it.
- `title` is a human-readable summary. It does not change per-request.
- `detail` is human-readable and may vary per-request. It should name what failed and why.
- `instance` is the request URI. Always included.
- Additional fields (e.g., `validation_errors` for 422) are allowed; they are documented
  alongside the `type` URI.

Internal server errors (5xx) include `type` and `title` but **never** include stack traces
or internal error details in the response. Those go in the server logs at ERROR level.

### 5. Health endpoints

Two endpoints, both unauthenticated:

| Endpoint | Check | Success | Failure |
|----------|-------|---------|---------|
| `GET /healthz` | Server process is alive | 200 `{"status":"ok"}` | — |
| `GET /readyz` | Server + database reachable | 200 `{"status":"ok","checks":{"db":"ok"}}` | 503 `{"status":"degraded","checks":{"db":"error: ..."}}` |

Load balancers and process supervisors use `/healthz`. Deployment orchestrators that
want to know if the service is ready to take traffic use `/readyz`.

### 6. Log level conventions

| Level | When to use |
|-------|-------------|
| `ERROR` | Unrecoverable condition requiring operator intervention (corruption, fatal config error, panic) |
| `WARN` | Degraded operation; retryable errors; client sent something invalid that wasn't caught by validation |
| `INFO` | Normal significant events: server start/stop, migration runs, authentication events |
| `DEBUG` | Detailed request/response data, SQL queries, intermediate state — never emitted in production JSON mode by default |
| `TRACE` | Reserved for hot-path diagnostics; never enabled in production |

The default filter is `INFO` in production and `DEBUG` for the local crate in development.
`RUST_LOG` overrides both.

## Consequences

- Every API endpoint emits consistent, correlated log records with no per-endpoint setup
- Error responses are machine-parseable by clients without bespoke parsing
- Log format is a one-line config change between dev and production
- Health endpoints are required before any deployment tooling is written
