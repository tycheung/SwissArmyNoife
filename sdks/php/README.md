# PHP SDK (`sak335`)

Composer package `swissarmynoife/sdk` — HTTP admin (`SakClient`) and Streamable HTTP MCP
(`SakMcpClient`) for SwissArmyNoife.

## Test

```bash
cd SwissArmyNoife/sdks/php
composer install
composer test
```

## Usage

```php
<?php
require 'vendor/autoload.php';

use SwissArmyNoife\Sdk\SakClient;

$sak = new SakClient('http://127.0.0.1:8787');
$health = $sak->health();
$modules = $sak->listModules();
```

### MCP

```php
use SwissArmyNoife\Sdk\SakMcpClient;

$mcp = new SakMcpClient('http://127.0.0.1:8080/mcp');
$mcp->setToken(getenv('MCP_HTTP_TOKEN') ?: null);
$pong = $mcp->ping();
```

Set `setAutoInitialize(false)` in unit tests that mock a single RPC.

## Examples

See [`examples/quickstart.php`](examples/quickstart.php).

Broker quickstart index: [`../../docs/sdk-quickstart.md`](../../docs/sdk-quickstart.md).
