#!/usr/bin/env bash
# 模拟「小桃子」发放 token 并激活 tzWarp
set -euo pipefail
export PATH="${HOME}/.cargo/bin:/opt/homebrew/bin:/usr/bin:/bin:${PATH}"

TOKEN="${1:-${TEAM_RELAY_API_KEY:-${TZAI_API_KEY:-${XIAOTAOZI_TOKEN:-}}}}"
if [[ -z "$TOKEN" ]]; then
  echo "用法: $0 <token>"
  echo "或先 export TZAI_API_KEY / XIAOTAOZI_TOKEN"
  exit 1
fi

# 1) 写入本机 token 文件（分发侧也可直接写这个文件）
mkdir -p "${HOME}/.tzworp"
printf '%s\n' "$TOKEN" > "${HOME}/.tzworp/token"
chmod 600 "${HOME}/.tzworp/token" 2>/dev/null || true
echo "已写入 ~/.tzworp/token"

# 2) 尝试深链激活（若 tzWarp 已在运行且注册了 tzworp scheme）
OPEN_URL="tzworp://activate?token=$(python3 -c "import urllib.parse,sys; print(urllib.parse.quote(sys.argv[1], safe=''))" "$TOKEN")"
echo "深链: tzworp://activate?token=***"
if command -v open >/dev/null 2>&1; then
  open "$OPEN_URL" 2>/dev/null || true
fi

echo "完成。启动/重启 tzWarp 后，智能体将使用该 token 访问中转站。"
echo "  ./script/run-tzworp"
