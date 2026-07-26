import 'dart:convert';

import 'package:http/http.dart' as http;

/// HTTP admin client (sak338-b).
class SakClient {
  SakClient(String? baseUrl, {http.Client? httpClient})
      : baseUrl = _strip((baseUrl == null || baseUrl.trim().isEmpty)
            ? 'http://127.0.0.1:8787'
            : baseUrl),
        _http = httpClient ?? http.Client();

  final String baseUrl;
  final http.Client _http;

  static String _strip(String u) {
    var out = u;
    while (out.endsWith('/')) {
      out = out.substring(0, out.length - 1);
    }
    return out;
  }

  Future<Map<String, dynamic>> health() => _getJson('/health');
  Future<Map<String, dynamic>> listModules() => _getJson('/v1/sak/modules');
  Future<Map<String, dynamic>> getModule(String id) =>
      _getJson('/v1/sak/modules/${Uri.encodeComponent(id)}');
  Future<Map<String, dynamic>> capacity() => _getJson('/v1/sak/capacity');
  Future<Map<String, dynamic>> listWork() => _getJson('/v1/sak/compute/work');
  Future<Map<String, dynamic>> listNodes() => _getJson('/v1/sak/compute/nodes');

  Future<Map<String, dynamic>> computeWork(Map<String, dynamic> body) =>
      _postJson('/v1/sak/compute/work', body);
  Future<Map<String, dynamic>> computeNodes(Map<String, dynamic> body) =>
      _postJson('/v1/sak/compute/nodes', body);

  Future<Map<String, dynamic>> enqueueWork(String kind,
          [Map<String, dynamic>? payload]) =>
      computeWork({
        'action': 'enqueue',
        'kind': kind,
        'payload': payload ?? <String, dynamic>{},
      });

  Future<Map<String, dynamic>> claimWork(String nodeId) =>
      computeWork({'action': 'claim', 'node_id': nodeId});

  Future<Map<String, dynamic>> completeWork(String workId, String nodeId,
          [Map<String, dynamic>? result]) =>
      computeWork({
        'action': 'complete',
        'work_id': workId,
        'node_id': nodeId,
        'result': result ?? <String, dynamic>{},
      });

  Future<Map<String, dynamic>> getWork(String workId) =>
      computeWork({'action': 'get', 'work_id': workId});

  Future<Map<String, dynamic>> requeueWork(String workId) =>
      computeWork({'action': 'requeue', 'work_id': workId});

  Future<Map<String, dynamic>> listWorkFiltered(
          [Map<String, dynamic>? filters]) =>
      computeWork({'action': 'list', ...?filters});

  Future<Map<String, dynamic>> listNodesFiltered(
          [Map<String, dynamic>? filters]) =>
      computeNodes({'action': 'list', ...?filters});

  Future<Map<String, dynamic>> _getJson(String path) async {
    final res = await _http.get(Uri.parse('$baseUrl$path'));
    if (res.statusCode < 200 || res.statusCode >= 300) {
      throw StateError('${res.statusCode}: ${res.body}');
    }
    return jsonDecode(res.body) as Map<String, dynamic>;
  }

  Future<Map<String, dynamic>> _postJson(
      String path, Map<String, dynamic> payload) async {
    final res = await _http.post(
      Uri.parse('$baseUrl$path'),
      headers: {'content-type': 'application/json'},
      body: jsonEncode(payload),
    );
    if (res.statusCode < 200 || res.statusCode >= 300) {
      throw StateError('${res.statusCode}: ${res.body}');
    }
    return jsonDecode(res.body) as Map<String, dynamic>;
  }
}
