# Enterprise runtime slots (`sak158`)

Marketplace module manifests may declare an optional hosted execution slot for enterprise tiers. OSS broker + marketplace **reject** these slots at validate time.

## `EnterpriseRuntimeSlot`

Defined in `module-manifest` (`enterprise.rs`):

| Variant | TOML / JSON | Meaning |
|---------|-------------|---------|
| `K8s` | `"k8s"` | Kubernetes-hosted module runtime (enterprise) |
| `E2b` | `"e2b"` | E2B sandbox slot (enterprise) |

Manifest field (optional):

```toml
enterprise_slot = "k8s"   # or "e2b"
```

## OSS marketplace deny

`validate_manifest` returns `module.incompatible` when `enterprise_slot` is set and `allowed_for_marketplace_oss()` is false (both variants return false today).

OSS installs use **wasm** or **process** runtimes only (`runtime = "wasm"` default). Real k8s/e2b orchestration is an enterprise follow-up — not required for open-source broker or community module publish.

## Related

- Workspace bind mounts for Docker jail: [workspace-bind-mounts.md](workspace-bind-mounts.md)
- Runtime kinds: `module-manifest` `RuntimeKind` (`wasm`, `process`; `native` also denied in OSS)
