#!/usr/bin/env python3
"""Quickstart sketch (`sak324-b`) — run against a live `http-admin`.

```bash
cargo run -p http-admin
python examples/quickstart.py
# or: SAK_HTTP=http://127.0.0.1:8787 python examples/quickstart.py
```
"""

from __future__ import annotations

import os

from swissarmynoife import SakClient


def main() -> None:
    base = os.environ.get("SAK_HTTP", "http://127.0.0.1:8787")
    with SakClient(base) as sak:
        health = sak.health()
        modules = sak.list_modules()
        print("health:", health)
        print("modules:", modules)


if __name__ == "__main__":
    main()
