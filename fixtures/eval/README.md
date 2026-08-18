# Eval fixtures (`sak535-a`)

Hand-authored `eval.run` cases under this directory (not `fixtures/offers/`).
Envelope: `sak.fixture.offer/v0`.

| File | Assert / case |
|------|----------------|
| `eq.pass.json` | `eq` pass |
| `json_path.pass.json` | `json_path` pointer + equals |
| `regex.pass.json` | `regex` match |
| `regex.invalid.json` | invalid pattern → `schema.invalid` |
| `numeric_tolerance.pass.json` | abs epsilon |
| `contains_all.pass.json` | array membership |
| `empty-checks.json` | empty `checks` → `schema.invalid` |
| `malformed-op.json` | unknown `op` → `schema.invalid` |
| `partial-fail.json` | mixed pass/fail per-check results |

Loaded by `crates/offer-eval/tests/golden_eval.rs` (`sak535-b`).
