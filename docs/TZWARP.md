# tzWarp 规范

给人和 Agent 的常驻约定。流程类操作走 `.agents/skills/tzworp-*`，不要把步骤再抄一份到这里。

接官方更新： [UPSTREAM.md](UPSTREAM.md)。

## 产品

- 终端是产品，小桃子是钥匙。
- **我们用不了的就不弄**；做的必须能当日常驱动：中文界面，智能体能流式、多轮、带上下文、命令确认后执行。
- 中文硬编码，不做 i18n。
- 不要 Warp 登录、升级引导、假账号。
- 不要 Warp Drive、Cloud Agent、云端交接；不要做假入口。
- `BindingDescription::new` 会把英文标题首字母大写；中文文案不要走这条路径当「翻译」。

## 标识

| | |
|---|---|
| 二进制 | `tzworp` |
| URL scheme | `tzworp://` |
| Bundle / AppId | `cool.kdp.tzWarp` |
| 密钥文件 | `~/.tzworp/token`（`0600`） |
| 申请账号 | `https://tzai.kdp.cool` |
| 中转 | `https://tzai.kdp.cool/v1` |
| 环境变量 | `TZAI_API_KEY` / `TEAM_RELAY_API_KEY` / `XIAOTAOZI_TOKEN` / `TEAM_TOKEN` |
| Cargo | `default` 必须含 `team_relay`（它绑定 `skip_login`） |

## 仓库

| 远程 / 分支 | 用途 |
|---|---|
| `origin` → `warpdotdev/Warp` | 官方 |
| `github` → `kedoupi/tzWarp` | 公开产品仓 |
| `tzworp` | 日常开发 |
| `github-main` | 只给 GitHub 用的孤儿快照 |
| `tzworp-before-upstream-YYYYMMDD` | 接官方前的备份 |

- 开发永远在 `tzworp`。**不要**把 `origin/master` merge 进 `github-main`。
- 推 `tzworp` 到 GitHub 会撞官方 LFS 历史洞；发布走孤儿快照（skill：`tzworp-publish`）。
- `README.md` 默认英文；中文在 `README.zh-CN.md`，页头互链。不要再把中英塞进同一篇。

## 测试与磁盘

- 日常**不要**跑官方 `cargo nextest --workspace`。它会把 `target` 顶到几十 GB。
- 产品改动测：`cargo test -p warp --lib --features team_relay -- ai::team_relay terminal::input::slash_commands::tests`
- 必须跑官方全量时用独立 `CARGO_TARGET_DIR`，跑完立刻删。见 [UPSTREAM.md](UPSTREAM.md)。

## 智能体（摘要）

细节和改代码步骤见 skill `tzworp-relay`。

- 客户端直连中转，不经 Warp Server。中转无状态。
- 只有 MCP 走 OpenAI `role=tool`。确认执行的 shell 结果必须是 assistant 文本。
- 终端里 `/plan` 必须拦截进智能体，不能进 zsh。
- 确认执行后续写必须带续写提示，并截住复读。
