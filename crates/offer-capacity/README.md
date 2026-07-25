# offer-capacity

`capacity.probe`, `capacity.pressure`, and `capacity.fit` offers plus hardware probe helpers.

## Probe selection (`CAPACITY_PROBE`)

| Value | Implementation | Behavior |
|-------|----------------|----------|
| *(unset)* | `LocalSysProbe` | Local sysinfo snapshot |
| `fake` | `FakeProbe` | Deterministic test snapshot |
| `ssh` | `SshFleetProbe` | **Stub** — always `provider.unreachable` |

## SSH fleet probe (`sak275`)

`SshFleetProbe` is a placeholder for enterprise/curated tiers that aggregate remote host signals over SSH. Setting `CAPACITY_PROBE=ssh` selects the stub via `probe_from_env()`.

By design the stub does **not** open SSH sessions in OSS builds:

- `HardwareProbe::probe()` → `ErrorCode::ProviderUnreachable`
- `stub_payload()` → `{ "ok": false, "reason": "ssh_probe_stub" }`

A real SSH fleet implementation is out of scope for the OSS stub.
Conformance and MCP smoke tests use `CAPACITY_PROBE=fake`.

### Inventory sketch (`sak275-f`–`j` / `sak275-k`)

Optional UTF-8 YAML sketch path (no SSH):

| Env / API | Role |
|-----------|------|
| `CAPACITY_SSH_INVENTORY` | Path to inventory file |
| `inventory_path_from_env` / `unique_host_ids_from_env` | Load + reject duplicate `- id:` lines |

See follow-up doc for the YAML sketch format. OSS stub still does not open SSH.

### Meminfo parse helpers (`sak275-l`)

Offline `/proc/meminfo` → `HardwareSnapshot` (no sockets):

| API | Role |
|-----|------|
| `parse_meminfo_kb` | `MemTotal` / `MemAvailable` (kB; free+buffers+cached fallback) |
| `hardware_snapshot_from_meminfo` | Map to `HardwareSnapshot` MB fields |
| `SshFleetProbe::snapshot_from_meminfo` | Same mapping with `source = "ssh:{host_id}"` |

`probe()` stays unreachable until live SSH (`CAPACITY_SSH_HOSTS` TODO).
