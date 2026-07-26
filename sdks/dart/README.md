# Dart SDK (`sak338`)

Pub package `swissarmynoife` — HTTP admin (`SakClient`) and Streamable HTTP MCP
(`SakMcpClient`) for SwissArmyNoife.

## Test

```bash
cd SwissArmyNoife/sdks/dart
dart pub get
dart test
```

## Usage

```dart
import 'package:swissarmynoife/swissarmynoife.dart';

final sak = SakClient('http://127.0.0.1:8787');
final health = await sak.health();
final modules = await sak.listModules();
```

### MCP

```dart
final mcp = SakMcpClient('http://127.0.0.1:8080/mcp')
  ..token = String.fromEnvironment('MCP_HTTP_TOKEN', defaultValue: '');
final pong = await mcp.ping();
```

Set `autoInitialize = false` in unit tests that mock a single RPC.

## Examples

See [`example/quickstart.dart`](example/quickstart.dart).

Broker quickstart index: [`../../docs/sdk-quickstart.md`](../../docs/sdk-quickstart.md).
