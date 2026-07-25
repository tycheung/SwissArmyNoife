#!/usr/bin/env bash
# SwissArmyNoife publish dry-run helper (sak326-b)
# Usage: from SwissArmyNoife/:  ./scripts/publish_dry_run.sh
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

echo "== cargo publish dry-run (Rust crates) =="
for crate in types sdk module-manifest module-registry; do
  echo "-- cargo publish -p ${crate} --dry-run --"
  cargo publish -p "${crate}" --dry-run
done

echo ""
echo "== npm dry-run (TypeScript SDK) =="
echo "cd ${ROOT}/sdks/typescript"
echo "npm ci && npm run build && npm publish --dry-run"
echo "(run the above manually or in CI before release)"

echo ""
echo "== PyPI build (Python SDK) =="
echo "cd ${ROOT}/sdks/python"
echo "python -m pip install --upgrade build && python -m build"
echo "(inspect dist/; twine upload only after sign-off)"

echo ""
echo "publish_dry_run: OK (Rust dry-runs passed; see docs/publish-dry-run.md for full checklist)"
