# Workspace bind mounts (`sak157`)

Docker sandbox backends may attach extra host directories into the jail via binding policy. Types live in `offer-sandbox` (`BindMount`, `WorkspaceMountPolicy`).

## Policy wire (binding JSON)

```json
{
  "mounts": [
    {
      "host": "/data/project",
      "guest": "workspace/src",
      "read_only": true
    }
  ]
}
```

| Field | Meaning |
|-------|---------|
| `host` | Absolute or relative host path to bind |
| `guest` | **Relative** path inside the jail (no leading `/`) |
| `read_only` | When true, Docker receives `:ro` on the `-v` spec |

Validation runs before `docker run`. Failures map to `schema.invalid` (bad shape) or `sandbox.violation:path_escape` (guest `..` escapes jail root).

## Docker `-v` wire

For each mount after the primary jail volume, the backend emits:

```text
docker run --rm \
  -v <jail_root>:/sak \
  -v <host>:/sak/<guest>[:ro] \
  -w /sak/<cwd> \
  <image> <argv...>
```

Guest paths are joined under the container root `/sak`. Read-only mounts append `:ro` to the bind spec.

## OSS default

Bind-mount policy is optional; empty `mounts` is valid. Enterprise-only hosted runtimes (`k8s`, `e2b`) are separate — see [enterprise-runtime-slots.md](enterprise-runtime-slots.md).
