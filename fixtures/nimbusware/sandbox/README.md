# Sandbox / jail golden fixtures (`sak159`)

Expected violation shapes for Nimbusware dual-run parity on **filesystem jail** cases.
Shared envelope: `sak.fixture.nimbusware/v0` (see [`../README.md`](../README.md)).

| File | Case |
|------|------|
| `path-escape.json` | `cwd: ".."` → `sandbox.violation:path_escape` |
| `argv-empty.json` | empty `argv` → `schema.invalid` (`argv must be non-empty`) |
| `absolute-cwd-escape.json` | absolute `cwd` outside jail → `sandbox.violation:path_escape` |
