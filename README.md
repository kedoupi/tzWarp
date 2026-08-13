# tzWarp

基于官方 [Warp](https://github.com/warpdotdev/Warp)（AGPL-3.0）的小团队定制终端。

- **中转站直连**：`https://tzai.kdp.cool/v1`（OpenAI 兼容）
- **无需 Warp 账号**（`skip_login` / `team_relay`）
- **小桃子 Token**：`tzworp://activate?token=…` / `~/.tzworp/token` / 环境变量
- **设置精简**：智能体 / 外观 / 功能 / 快捷键 / 关于
- **品牌**：`tzWarp`（bundle id: `cool.kdp.tzWarp`）

官方原 README 见 [README.upstream.md](README.upstream.md)。

## 快速开始

### 1. 依赖

```bash
./script/bootstrap --skip-common-skills   # 首次
```

### 2. API Key

```bash
export TZAI_API_KEY='你的key'
# 或
export TEAM_RELAY_API_KEY='你的key'
```

可选覆盖 Base URL：

```bash
export TEAM_RELAY_BASE_URL='https://tzai.kdp.cool/v1'
```

### 3. 运行

```bash
./script/run-tzworp
# 或
cargo run -p warp --bin tzworp --features team_relay
```

应用数据目录与官方 Warp 隔离，可并存。

### 4. 使用

1. 启动后打开智能体输入
2. 模型列表来自中转站 `GET /v1/models`
3. 提问即可；请求只打到中转站

也可在 **设置 → 智能体** 中粘贴小桃子 API 密钥。

模拟发 Token：

```bash
./scripts/activate_demo.sh "$TZAI_API_KEY"
```

## 架构

```
tzWarp 客户端 ──► https://tzai.kdp.cool/v1/chat/completions
                 https://tzai.kdp.cool/v1/models
```

不经过 Warp Server。官方完整 Oz 云编排未开源，本产品提供中转对话级 Agent 能力。

## 许可证

遵循上游 AGPL-3.0（及 UI 框架 MIT 部分）。二次分发请提供对应源码。
