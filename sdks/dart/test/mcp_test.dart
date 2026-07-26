import 'dart:convert';

import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:swissarmynoife/swissarmynoife.dart';
import 'package:test/test.dart';

void main() {
  test('ping negotiates session', () async {
    var n = 0;
    final mock = MockClient((req) async {
      final body = jsonDecode(req.body) as Map<String, dynamic>;
      final method = body['method'] as String;
      n++;
      switch (method) {
        case 'initialize':
          return http.Response(jsonEncode({'jsonrpc': '2.0', 'id': 1, 'result': {}}),
              200, headers: {'mcp-session-id': 'sess-dart-1'});
        case 'notifications/initialized':
          return http.Response('', 202);
        case 'tools/call':
          expect(req.headers['mcp-session-id'], 'sess-dart-1');
          return http.Response(
              jsonEncode({
                'jsonrpc': '2.0',
                'id': 2,
                'result': {
                  'content': [
                    {'type': 'text', 'text': 'pong'}
                  ]
                }
              }),
              200);
        default:
          return http.Response('bad', 500);
      }
    });
    final mcp = SakMcpClient('http://example.test/mcp', httpClient: mock);
    expect(await mcp.ping(), 'pong');
    expect(mcp.sessionId, 'sess-dart-1');
    expect(n, greaterThanOrEqualTo(3));
  });

  test('toolsList no auto init', () async {
    final mock = MockClient((req) async {
      final body = jsonDecode(req.body) as Map<String, dynamic>;
      expect(body['method'], 'tools/list');
      return http.Response(
          jsonEncode({
            'jsonrpc': '2.0',
            'id': 1,
            'result': {'tools': []}
          }),
          200);
    });
    final mcp = SakMcpClient('http://example.test/mcp', httpClient: mock)
      ..autoInitialize = false;
    final out = await mcp.toolsList() as Map;
    expect(out.containsKey('tools'), true);
  });

  test('catalogList', () async {
    final mock = MockClient((req) async {
      final body = jsonDecode(req.body) as Map<String, dynamic>;
      switch (body['method'] as String) {
        case 'initialize':
          return http.Response(jsonEncode({'jsonrpc': '2.0', 'id': 1, 'result': {}}),
              200, headers: {'mcp-session-id': 's2'});
        case 'notifications/initialized':
          return http.Response('', 202);
        default:
          return http.Response(
              jsonEncode({
                'jsonrpc': '2.0',
                'id': 2,
                'result': {'offers': []}
              }),
              200);
      }
    });
    final out =
        await SakMcpClient('http://example.test/mcp', httpClient: mock)
            .catalogList() as Map;
    expect(out.containsKey('offers'), true);
  });
}
