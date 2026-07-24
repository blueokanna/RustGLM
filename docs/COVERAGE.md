# Test Coverage

This document records the coverage policy and the latest locally verified snapshot. CI regenerates
the report for every revision, enforces the line threshold, and uploads the complete LCOV data.

## Current snapshot

Measured on 2026-07-24 from the working tree with:

```bash
cargo coverage
```

| Metric | Covered | CI minimum |
| --- | ---: | ---: |
| Regions | 92.72% | Informational |
| Functions | 88.33% | Informational |
| Lines | 94.02% | 90.00% |

The run executed 94 offline tests successfully. Two credentialed live tests were discovered and
ignored by design. All 36 example binaries are compiled by `--all-targets`; their `main` functions are not
executed by the coverage run because they can consume provider quota or mutate caller-selected
resources.

## Module line coverage

| Module | Lines |
| --- | ---: |
| `agent.rs` | 96.22% |
| `auth.rs` | 94.74% |
| `client.rs` | 97.95% |
| `error.rs` | 66.67% |
| `mcp.rs` | 51.60% |
| `memory.rs` | 94.26% |
| `model.rs` | 87.44% |
| `provider.rs` | 100.00% |
| `rag.rs` | 96.63% |
| `realtime.rs` | 94.15% |
| `realtime/events.rs` | 96.19% |
| `tool_stream.rs` | 93.93% |
| `transport.rs` | 96.17% |
| `types.rs` | 93.36% |
| `voice.rs` | 96.36% |

The MCP module is lower because successful protocol operations require a running, initialized MCP
peer. Offline tests cover configuration, secret redaction, URL/header validation, and pre-I/O
failure paths. CI still measures MCP as part of the all-feature total; it is not excluded from the
threshold.

## Commands

```bash
# Print the all-feature summary and enforce 90% line coverage.
cargo coverage

# Generate target/rustglm-lcov.info and enforce the same threshold.
cargo coverage-lcov

# Generate an inspectable HTML report.
cargo llvm-cov --locked --all-features --all-targets --html
```

Live tests remain separate because they need credentials and may incur charges:

```powershell
$env:ZHIPU_API_KEY = "key_id.secret"
cargo test --test live_zhipu -- --ignored --nocapture
cargo test --test live_realtime -- --ignored --nocapture
```

Coverage percentages are snapshots, not permanent claims. Use the CI artifact or rerun the command
for the exact revision being evaluated.
