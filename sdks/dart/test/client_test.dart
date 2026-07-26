import 'dart:convert';

import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:swissarmynoife/swissarmynoife.dart';
import 'package:test/test.dart';

void main() {
  test('baseUrl strips slash', () {
    final c = SakClient('http://127.0.0.1:8787/');
    expect(c.baseUrl, 'http://127.0.0.1:8787');
  });

  test('health', () async {
    final mock = MockClient((req) async {
      expect(req.url.path, '/health');
      return http.Response(jsonEncode({'ok': true}), 200);
    });
    final c = SakClient('http://example.test', httpClient: mock);
    expect((await c.health())['ok'], true);
  });

  test('list helpers', () async {
    final cases = {
      'listModules': ['/v1/sak/modules', {'modules': []}],
      'listWork': ['/v1/sak/compute/work', {'work': []}],
      'listNodes': ['/v1/sak/compute/nodes', {'nodes': []}],
      'capacity': [
        '/v1/sak/capacity',
        {
          'snapshot': {'total_ram_mb': 1}
        }
      ],
    };
    for (final entry in cases.entries) {
      final path = entry.value[0] as String;
      final body = entry.value[1] as Map<String, dynamic>;
      final mock = MockClient((req) async {
        expect(req.url.path, path);
        return http.Response(jsonEncode(body), 200);
      });
      final c = SakClient('http://example.test', httpClient: mock);
      final out = await switch (entry.key) {
        'listModules' => c.listModules(),
        'listWork' => c.listWork(),
        'listNodes' => c.listNodes(),
        _ => c.capacity(),
      };
      expect(out, isA<Map<String, dynamic>>());
    }
  });

  test('enqueue and claim', () async {
    final actions = <String>[];
    final mock = MockClient((req) async {
      final body = jsonDecode(req.body) as Map<String, dynamic>;
      actions.add(body['action'] as String);
      return http.Response(
          jsonEncode({
            'action': 'ok',
            'work': {'id': 'w1'}
          }),
          200);
    });
    final c = SakClient('http://example.test', httpClient: mock);
    await c.enqueueWork('echo', {'n': 1});
    await c.requeueWork('w1');
    await c.claimWork('n1');
    await c.completeWork('w1', 'n1');
    await c.getWork('w1');
    expect(actions, ['enqueue', 'requeue', 'claim', 'complete', 'get']);
  });
}
