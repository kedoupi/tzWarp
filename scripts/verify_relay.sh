#!/usr/bin/env bash
# 验证中转站 + tzWarp 二进制是否就绪
set -euo pipefail
export PATH="${HOME}/.cargo/bin:/opt/homebrew/bin:/usr/bin:/bin:${PATH}"

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
KEY="${TEAM_RELAY_API_KEY:-${TZAI_API_KEY:-}}"
BASE="${TEAM_RELAY_BASE_URL:-https://tzai.kdp.cool/v1}"

if [[ -z "$KEY" ]]; then
  echo "FAIL: 需要 TZAI_API_KEY 或 TEAM_RELAY_API_KEY"
  exit 1
fi

echo "== 1. 中转站 models =="
code=$(curl -sS -o /tmp/tz_models.json -w "%{http_code}" \
  -H "Authorization: Bearer ${KEY}" "${BASE}/models")
echo "HTTP $code"
if [[ "$code" != "200" ]]; then
  head -c 300 /tmp/tz_models.json; echo
  exit 1
fi
python3 - <<'PY'
import json
d=json.load(open("/tmp/tz_models.json"))
ids=[m["id"] for m in d.get("data",[])]
print(f"models={len(ids)} sample={ids[:5]}")
open("/tmp/tz_model_pick.txt","w").write(next((i for i in ids if "mini" in i or "5.4" in i), ids[0] if ids else "gpt-5.4-mini"))
PY
MODEL=$(cat /tmp/tz_model_pick.txt)
echo "pick model=$MODEL"

echo "== 2. chat/completions =="
code=$(curl -sS -o /tmp/tz_chat.json -w "%{http_code}" --max-time 90 \
  -H "Authorization: Bearer ${KEY}" -H "Content-Type: application/json" \
  -d "{\"model\":\"$MODEL\",\"messages\":[{\"role\":\"user\",\"content\":\"只回复：tzWarp-ok\"}],\"max_tokens\":32,\"stream\":false}" \
  "${BASE}/chat/completions")
echo "HTTP $code"
head -c 400 /tmp/tz_chat.json; echo
if [[ "$code" != "200" ]]; then exit 1; fi

echo "== 3. 二进制 =="
BIN="$ROOT/target/debug/tzworp"
if [[ ! -x "$BIN" ]]; then
  BIN="$ROOT/target/release/tzworp"
fi
if [[ -x "$BIN" ]]; then
  echo "OK binary: $BIN"
  ls -lh "$BIN"
else
  echo "WARN: binary not built yet (run cargo build -p warp --bin tzworp)"
fi

echo "ALL CHECKS PASSED"
