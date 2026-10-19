#!/bin/bash
set -euo pipefail
aircastd=$1
origin=${2:-http://localhost:8781}
out="$(cd "$(dirname "$0")/.." && pwd)/app/src/test/resources/aircastd"
mkdir -p "$out"
for page in aircraft telemetry connect flights device engineering video recordings network cameras setup; do
    curl -fsS "$origin/api/views/$page" | python3 -m json.tool --indent 2 > "$out/$page.json"
done
grep -rhoE --include='*.go' --exclude='*_test.go' 'json:"[A-Za-z_][A-Za-z0-9_]*' "$aircastd/internal" "$aircastd/cmd" | sed 's/^json:"//' | sort -u > "$out/keys.txt"
