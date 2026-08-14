<p align="center">
  <img src="images/tzworp-icon.png" width="160" alt="tzWarp">
</p>

<h1 align="center">tzWarp</h1>

<p align="center">
  <strong>小桃子旗下的中文 AI 终端</strong><br>
  基于开源 <a href="https://github.com/warpdotdev/Warp">Warp</a> 客户端，不走 Warp 云，直连团队自己的中转站。
</p>

<p align="center">
  <a href="#tzwarp">中文</a> ·
  <a href="#tzwarp-english">English</a>
  &nbsp;·&nbsp;
  <img alt="version" src="https://img.shields.io/badge/version-1.0.0-orange">
  <img alt="license" src="https://img.shields.io/badge/license-AGPL--3.0-blue">
  <img alt="platform" src="https://img.shields.io/badge/macOS-12%2B%20Apple%20Silicon-black">
</p>

官方原 README 见 [README.upstream.md](README.upstream.md)。接官方更新见 [docs/UPSTREAM.md](docs/UPSTREAM.md)。

---

# tzWarp

终端是产品。小桃子是钥匙。

tzWarp 把官方 Warp 开源客户端做成小团队能天天用的中文终端：**块状输出、补全、分屏、主题都还在**；智能体只跟你们自己的 OpenAI 兼容中转说话，不登录 Warp，也不把会话送去 Warp 云。

当前默认中转：[`https://tzai.kdp.cool/v1`](https://tzai.kdp.cool/v1)（小桃子）。

## 为什么存在

官方 Warp 很强，但它的完整 Agent / Drive / 云端编排绑在 Warp 云上。很多团队只想要：

- 一台真正能当日常驱动的终端
- 中文界面，不用在设置里翻英文
- 一个 Key 接通自己的模型中转
- 命令先确认再跑，别偷偷执行

官方开源了客户端，没开源云。tzWarp 就做客户端这一侧：**用得上的做成，用不上的不装。**

## 1.0.0 能做什么

| | |
|---|---|
| **日常终端** | Warp 同级的 GPU 终端：块选择、补全、分屏、主题、工作区 |
| **中文界面** | 设置、菜单、欢迎、斜杠命令说明默认中文 |
| **中转智能体** | 流式输出、多轮对话、工作区上下文、本地 `AGENTS.md` / `WARP.md` |
| **斜杠命令** | 终端里输入 `/plan` 会进智能体，不会丢给 zsh |
| **确认执行** | 模型给出的 shell 先出卡片，你点了才跑；结果回到对话 |
| **MCP** | 已配置的 MCP 工具走 function calling |
| **无 Warp 账号** | 不登录、不升级引导、不伪装官方账号 |
| **与官方并存** | Bundle ID `cool.kdp.tzWarp`，数据目录独立 |

## 1.0.0 明确不做

这些能力依赖 Warp 云，本版本不提供、也不做假入口：

- Warp Drive / 云端知识库同步
- Cloud Agent / 云端交接
- Warp 登录、套餐、官方自动更新

Windows / Linux 安装包尚未提供，可从源码自行编译。

## 安装（macOS Apple Silicon）

1. 下载 [Releases](https://github.com/kedoupi/tzWarp/releases) 里的 `tzWarp-1.0.0-macos-arm64.dmg`
2. 把 **tzWarp** 拖进 **Applications**
3. 第一次打开：右键图标 → **打开**（安装包未经过 Apple 公证）

自行打包：

```bash
./script/package-tzworp
# 产物：../dist/tzWarp-1.0.0-macos-arm64.dmg（或本仓库 dist/）
```

## 接入小桃子

任选一种即可。

**设置里粘贴**

打开 tzWarp → **设置 → 智能体** → 填入小桃子 API 密钥。

**本地文件**（权限 `0600`）

```bash
mkdir -p ~/.tzworp
echo '你的密钥' > ~/.tzworp/token
chmod 600 ~/.tzworp/token
```

**环境变量**

```bash
export TZAI_API_KEY='你的密钥'
# 或 TEAM_RELAY_API_KEY / XIAOTAOZI_TOKEN / TEAM_TOKEN
```

**激活链接**

```
tzworp://activate?token=你的密钥
```

可选覆盖中转地址：

```bash
export TEAM_RELAY_BASE_URL='https://tzai.kdp.cool/v1'
```

## 五分钟上手

1. 打开 tzWarp，和平常一样用终端
2. 输入 `/plan 这是什么项目呢`，回车进智能体
3. 需要跑命令时会出现确认卡片，点了才执行
4. 接着问「做个总结」——同一段对话会带着刚才的输出继续

模型列表来自中转站 `GET /v1/models`。请求只打中转，不经过 Warp Server。

## 架构

```
tzWarp.app  ──►  https://tzai.kdp.cool/v1/chat/completions
            ──►  https://tzai.kdp.cool/v1/models
            ──►  本机 PTY / 确认后的 shell / MCP
```

中转站无状态。客户端负责拼历史、配好 tool 调用，以及把「确认执行」的结果以文本回灌，避免把 shell 输出误当成 OpenAI 的 `role=tool`。

## 从源码构建

```bash
./script/bootstrap --skip-common-skills   # 首次
export TZAI_API_KEY='你的密钥'
./script/run-tzworp
```

或：

```bash
cargo run -p warp --bin tzworp --features team_relay
```

接官方更新的步骤见 [docs/UPSTREAM.md](docs/UPSTREAM.md)。日常开发在 `tzworp` 分支。

## 许可证与致谢

本项目基于 [Warp](https://github.com/warpdotdev/Warp)，遵循 **AGPL-3.0**（UI 框架部分为 MIT）。二次分发必须提供对应源码。

版权与商标仍归各自所有者。tzWarp 不是 Warp 官方产品，也未获得 Denver Technologies, Inc. 的背书。

---

# tzWarp (English)

<p align="center"><strong>A Chinese AI terminal from Xiaotaozi (小桃子).</strong><br>
Forked from the open-source <a href="https://github.com/warpdotdev/Warp">Warp</a> client. No Warp Cloud. Talks only to your OpenAI-compatible relay.</p>

The terminal is the product. Xiaotaozi is the key.

tzWarp keeps what you actually use in Warp — GPU terminal, block select, completions, splits, themes — and replaces the cloud agent with a relay-backed chat that streams, keeps multi-turn context, and **confirms before it runs a command**.

Default relay: [`https://tzai.kdp.cool/v1`](https://tzai.kdp.cool/v1).

Official upstream README: [README.upstream.md](README.upstream.md). Ingesting Warp updates: [docs/UPSTREAM.md](docs/UPSTREAM.md).

## Why it exists

Warp’s client is open source. Warp’s cloud (Drive, Cloud Agent, the full Oz orchestrator) is not. Many small teams only need:

- A daily-driver terminal
- A Chinese UI
- One API key to their own models
- Commands that wait for a click

We ship what we use. We don’t fake the rest.

## What 1.0.0 includes

| | |
|---|---|
| **Terminal** | Warp-class GPU terminal: blocks, completions, splits, themes, workspaces |
| **Chinese UI** | Settings, menus, welcome, slash-command copy |
| **Relay agent** | Streaming, multi-turn, workspace context, local `AGENTS.md` / `WARP.md` |
| **Slash commands** | `/plan` in the terminal opens the agent — it is not sent to zsh |
| **Confirm-run** | Shell from the model is a card first; output returns to the thread |
| **MCP** | Configured MCP tools via function calling |
| **No Warp account** | No login, no upgrade nags, no fake official identity |
| **Side-by-side** | Bundle ID `cool.kdp.tzWarp`, separate data directory from official Warp |

## What 1.0.0 does not include

These need Warp Cloud. They are not implemented and are not shown as if they were:

- Warp Drive / cloud knowledge sync
- Cloud Agent / cloud handoff
- Warp login, billing, official auto-update

No Windows / Linux installers yet. Build from source if you need those platforms.

## Install (macOS Apple Silicon)

1. Grab `tzWarp-1.0.0-macos-arm64.dmg` from [Releases](https://github.com/kedoupi/tzWarp/releases)
2. Drag **tzWarp** into **Applications**
3. First launch: right-click → **Open** (the build is not Apple-notarized)

Build the installer:

```bash
./script/package-tzworp
# output: ../dist/tzWarp-1.0.0-macos-arm64.dmg (or ./dist)
```

## Connect Xiaotaozi

Any one of these works.

**In Settings**

tzWarp → **Settings → Agent** → paste the Xiaotaozi API key.

**Token file** (`0600`)

```bash
mkdir -p ~/.tzworp
echo 'your-key' > ~/.tzworp/token
chmod 600 ~/.tzworp/token
```

**Environment**

```bash
export TZAI_API_KEY='your-key'
# or TEAM_RELAY_API_KEY / XIAOTAOZI_TOKEN / TEAM_TOKEN
```

**URL scheme**

```
tzworp://activate?token=your-key
```

Optional relay override:

```bash
export TEAM_RELAY_BASE_URL='https://tzai.kdp.cool/v1'
```

## Five-minute tour

1. Use the terminal as usual
2. Type `/plan what is this project?` and press Enter
3. Confirm any command card before it runs
4. Ask for a summary — the same thread keeps the command output

Models come from `GET /v1/models` on the relay. Traffic never goes to Warp Server.

## Architecture

```
tzWarp.app  ──►  https://tzai.kdp.cool/v1/chat/completions
            ──►  https://tzai.kdp.cool/v1/models
            ──►  local PTY / confirmed shell / MCP
```

The relay is stateless. The client assembles history, pairs tool calls, and feeds confirm-run results back as assistant text so shell output is never mistaken for an OpenAI `role=tool` message.

## Build from source

```bash
./script/bootstrap --skip-common-skills   # first time
export TZAI_API_KEY='your-key'
./script/run-tzworp
```

Development happens on the `tzworp` branch.

## License and credits

Based on [Warp](https://github.com/warpdotdev/Warp), licensed under **AGPL-3.0** (parts of the UI stack are MIT). Downstream distributions must provide corresponding source.

Trademarks remain with their owners. tzWarp is not an official Warp product and is not endorsed by Denver Technologies, Inc.
