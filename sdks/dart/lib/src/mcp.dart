import 'dart:convert';

import 'package:http/http.dart' as http;

import 'sdk_info.dart';

/// Streamable HTTP MCP client (sak338-c).
class SakMcpClient {
  SakMcpClient(String? baseUrl, {http.Client? httpClient})
      : baseUrl = _strip((baseUrl == null || baseUrl.trim().isEmpty)
            ? 'http://127.0.0.1:8080/mcp'
            : baseUrl),
        _http = httpClient ?? http.Client();

  final String baseUrl;
  final http.Client _http;

  String? token;
  bool autoInitialize = true;

  int _rpcId = 0;
  String? _sessionId;
  bool _initialized = false;

  String? get sessionId => _sessionId;

  static String _strip(String u) {
    var out = u;
    while (out.endsWith('/')) {
      out = out.substring(0, out.length - 1);
    }
    return out;
  }

  Future<dynamic> initialize() async {
    final result = await _rpc('initialize', {
      'protocolVersion': '2024-11-05',
      'capabilities': <String, dynamic>{},
      'clientInfo': {'name': 'swissarmynoife-dart', 'version': SdkInfo.version},
    });
    await _post({'jsonrpc': '2.0', 'method': 'notifications/initialized'},
        notification: true);
    _initialized = true;
    return result;
  }

  Future<String> ping() async {
    return _extractPingText(await _toolsCall('ping'));
  }

  Future<dynamic> toolsList() async {
    await _ensureSession();
    return _rpc('tools/list');
  }

  Future<dynamic> catalogList() async => _toolsCall('catalog_list');

  Future<void> _ensureSession() async {
    if (!autoInitialize || _initialized) return;
    await initialize();
  }

  Future<dynamic> _toolsCall(String name,
      [Map<String, dynamic> arguments = const {}]) async {
    await _ensureSession();
    return _rpc('tools/call', {'name': name, 'arguments': arguments});
  }

  Future<dynamic> _rpc(String method,
      [Map<String, dynamic> params = const {}]) async {
    _rpcId++;
    final res = await _post({
      'jsonrpc': '2.0',
      'id': _rpcId,
      'method': method,
      'params': params,
    });
    final body = jsonDecode(res.body);
    _captureSession(res, body);
    if (body is Map && body.containsKey('error')) {
      final err = body['error'];
      final msg = err is Map ? (err['message'] ?? err) : err;
      throw StateError('MCP $method failed: $msg');
    }
    if (body is Map && body.containsKey('result')) return body['result'];
    return body;
  }

  Future<http.Response> _post(Map<String, dynamic> payload,
      {bool notification = false}) async {
    final headers = <String, String>{
      'content-type': 'application/json',
      'accept': 'application/json, text/event-stream',
    };
    if (token != null && token!.isNotEmpty) {
      headers['authorization'] = 'Bearer $token';
    }
    if (_sessionId != null && _sessionId!.isNotEmpty) {
      headers['mcp-session-id'] = _sessionId!;
    }
    final res = await _http.post(
      Uri.parse(baseUrl),
      headers: headers,
      body: jsonEncode(payload),
    );
    if (notification && (res.statusCode == 200 || res.statusCode == 202)) {
      return res;
    }
    if (res.statusCode < 200 || res.statusCode >= 300) {
      throw StateError('${res.statusCode}: ${res.body}');
    }
    return res;
  }

  void _captureSession(http.Response res, dynamic body) {
    if (_sessionId == null || _sessionId!.isEmpty) {
      final sid = res.headers['mcp-session-id'];
      if (sid != null && sid.trim().isNotEmpty) {
        _sessionId = sid.trim();
      }
    }
    if ((_sessionId == null || _sessionId!.isEmpty) && body is Map) {
      final fromBody = _sessionIdFromBody(body);
      if (fromBody != null) _sessionId = fromBody;
    }
  }

  static String? _sessionIdFromBody(Map body) {
    for (final key in ['sessionId', 'session_id', 'mcp-session-id']) {
      final v = body[key];
      if (v is String && v.trim().isNotEmpty) return v.trim();
    }
    final result = body['result'];
    if (result is Map) {
      for (final key in ['sessionId', 'session_id', 'mcp-session-id']) {
        final v = result[key];
        if (v is String && v.trim().isNotEmpty) return v.trim();
      }
    }
    return null;
  }

  static String _extractPingText(dynamic result) {
    if (result is String) return result;
    if (result is Map) {
      final content = result['content'];
      if (content is List) {
        for (final item in content) {
          if (item is Map && item['text'] is String) {
            return item['text'] as String;
          }
        }
      }
    }
    return '$result';
  }
}
