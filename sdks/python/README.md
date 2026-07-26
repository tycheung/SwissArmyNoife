# Python SDK (`sak322`)

HTTP admin client (`SakClient`) and Streamable HTTP MCP stub (`SakMcpClient`) for SwissArmyNoife.

## Install

```bash
cd SwissArmyNoife/sdks/python
pip install -e ".[dev]"
```

## Usage

```python
from swissarmynoife import SakClient

with SakClient("http://127.0.0.1:8787") as sak:
    print(sak.health())
    print(sak.list_modules())
    print(sak.capacity())
    print(sak.list_work())
    print(sak.list_nodes())
```

### MCP (`SakMcpClient`)

Streamable HTTP MCP client. On first tool call (or explicit ``initialize()``), negotiates a
session and sends ``mcp-session-id`` on later RPCs (``sak329-b``).

| Method | MCP wire | Description |
|--------|----------|-------------|
| `initialize()` | `initialize` + `notifications/initialized` | Session handshake |
| `ping()` | `tools/call` → `ping` | Health smoke; returns text from tool result |
| `tools_list()` | `tools/list` | Lists broker MCP tools |
| `catalog_list()` | `tools/call` → `catalog_list` | Lists catalog offers via MCP tool |

```python
from swissarmynoife import SakMcpClient

mcp = SakMcpClient("http://127.0.0.1:8080/mcp", token="optional-bearer")
print(mcp.ping())
print(mcp.tools_list())
print(mcp.catalog_list())
```

## Examples

See [`examples/quickstart.py`](examples/quickstart.py) for a minimal health + `list_modules` sketch
against a running `http-admin`:

```bash
cargo run -p http-admin
cd SwissArmyNoife/sdks/python
python examples/quickstart.py
```

Broker quickstart index: [`../../docs/sdk-quickstart.md`](../../docs/sdk-quickstart.md).  
Marketplace registry HTTP: [`../../docs/marketplace-quickstart.md`](../../docs/marketplace-quickstart.md).

## Tests

```bash
cd SwissArmyNoife/sdks/python
pytest -q
```

Tests mock `httpx` (no live admin or MCP server required).

MCP session negotiation (`initialize`) lands in a later slice; `ping` posts `tools/call` directly.
