import 'dart:io';

import 'package:swissarmynoife/swissarmynoife.dart';

Future<void> main() async {
  final base = Platform.environment['SAK_HTTP'] ?? 'http://127.0.0.1:8787';
  final sak = SakClient(base);
  print('health=${await sak.health()}');
  print('modules=${await sak.listModules()}');
}
