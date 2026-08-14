# 接官方 Warp 更新

开发永远在 `tzworp` 上做。`github-main` 只是推 GitHub 用的孤儿快照，**不要**往上面 merge `origin/master`。

| 远程 / 分支 | 用途 |
|---|---|
| `origin` → `warpdotdev/Warp` | 官方 |
| `github` → `kedoupi/tzWarp` | 产品仓 |
| `tzworp` | 日常开发和接官方 |
| `github-main` | 仅发布快照 |
| `tzworp-before-upstream-YYYYMMDD` | 每次升级前的备份 |

当前基线写在最近一次 merge commit 里。第一次实操：`42effe8` → `90fdba19`（官方 225 个提交，冲突 2 个文件）。

## 升级步骤

工作区必须干净。先提交自己的改动。

```bash
cd Warp
git checkout tzworp
git branch tzworp-before-upstream-$(date +%Y%m%d)

# 第一次或 shallow 克隆后：
git fetch --unshallow --prune origin   # 只需一次
# 之后：
git fetch origin

git log -1 --format='%h %ci %s' origin/master
git rev-list --left-right --count HEAD...origin/master   # 我们超前 / 官方超前
```

选稳定点，不要无脑跟每天的 `master` 尖。官方 stable tag 可能比我们的基线还旧（2026-08 已超过 2026-06 的 stable）。这时就合 `origin/master`，或合到某个你看过的 commit。

```bash
git merge origin/master
```

冲突处理：

- `app/src/ai/team_relay/`：留我们的
- `#[cfg(feature = "team_relay")]`：两边都留，官方新结构套我们的闸门
- 日常中文：留中文
- `Cargo.toml` default：必须还带 `team_relay`
- 纯官方新文件：用官方的
- 登录 / 计费 / Cloud Agent / Drive / Handoff：不要为了合进测试而去打开

合完先编产品，不要先跑官方全量测试：

```bash
cargo build -p warp --bin tzworp --features team_relay
cargo test -p warp --lib --features team_relay -- ai::team_relay terminal::input::slash_commands::tests
```

手测：新对话 `/plan`、确认执行一条命令、中转站还能回。

通过后再说要不要刷新 GitHub 快照。`tzworp` 带官方完整历史，推 `github/main` 以前因 LFS 对象洞失败过，所以发布仍可能要另打孤儿快照。

回滚：

```bash
git checkout tzworp
git reset --hard tzworp-before-upstream-YYYYMMDD
```

## 磁盘

官方 `cargo nextest --workspace` 会把 `Warp/target` 顶到几十 GB，这台机器已经因此把盘写满过一次。

- 日常不要跑官方全量 nextest。
- 必须跑时单独指定目录，跑完立刻删：

```bash
CARGO_TARGET_DIR=/tmp/tzworp-nextest cargo nextest run --no-fail-fast --workspace --exclude command-signatures-v2
rm -rf /tmp/tzworp-nextest
```

- 平时 `Warp/target` 胀了就 `rm -rf Warp/target`，只丢编译缓存。
