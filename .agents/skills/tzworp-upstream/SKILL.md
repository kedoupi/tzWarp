---
name: tzworp-upstream
description: >
  把官方 Warp 更新合进 tzWarp 的 tzworp 分支。步骤以 docs/UPSTREAM.md 为准。
  Use when the user asks 接官方, 合 origin/master, 升级 Warp, or /tzworp-upstream.
---

# tzworp-upstream

先完整读并执行 `docs/UPSTREAM.md`。不要另写一套步骤。

合完后的产品闸门（文档里有，这里只点名）：

- `app/src/ai/team_relay/` 留我们的
- `#[cfg(feature = "team_relay")]` 两边都留
- 日常中文留中文
- `Cargo.toml` `default` 必须还带 `team_relay`
- 登录 / 计费 / Cloud Agent / Drive / Handoff 不要为了合进测试而打开

合完先编 `tzworp`，再跑 `tzworp-relay` skill 里的那组测试，再手测 `/plan` + 确认执行。通过后才问要不要 `tzworp-publish`。
