# offer-memory

`memory.index`, `memory.search`, and `memory.embed` offers plus vector backends and SQLite index metadata.

## Vector backends (`BackendKind`)

| Kind | Catalog value | Implementation | Notes |
|------|---------------|----------------|-------|
| Exact | `exact` | `NaiveIndex` (brute-force cosine) | Default; deterministic tests |
| HNSW | `hnsw` | `HnswLite` (pure Rust NSW) | Approximate search (`sak222-b`) |
| FAISS | `faiss` | **`NaiveIndex` stand-in** | Catalogued alternative (`sak222b-a`); **FAISS FFI not shipped** |

Bind-time backend selection uses catalog metadata (`sak229-a`): `exact`, `hnsw`, or `faiss`.
At **bind time**, callers may select `faiss` like any other backend; until native FAISS FFI ships,
that choice **maps to the exact / `NaiveIndex` stub** — not a separate approximate index.

## FAISS / FFI deferred (`sak222b`, `sak222b-d`)

`BackendKind::Faiss` is registered in the catalog and wired through `DynIndex`, but OSS builds **do not** link native FAISS. Requests with `backend: "faiss"` use the same in-process exact index as a compatibility stub so golden fixtures and bind-time selection can be tested without FFI.

### Optional Cargo feature boundary (`sak222b-g`)

| Feature | Default | Behavior today |
|---------|---------|----------------|
| `faiss-ffi` | off | **Empty** feature — documents the future FFI boundary; does **not** link native FAISS |

```bash
cargo check -p offer-memory --features faiss-ffi
cargo test -p offer-memory --lib faiss
```

Expect: `BackendKind::Faiss` → `NaiveIndex` fallback; default workspace builds stay FFI-free.

A real FAISS provider (FFI or curated module) is a **future follow-up** — not required for v0 memory MCP (`memory_index` / `memory_search`).

See also [`docs/faiss-ffi-followup.md`](../../../docs/faiss-ffi-followup.md) (FAISS FFI planned follow-up).
