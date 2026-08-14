---
name: tzworp-chinese-ui
description: >
  收 tzWarp 日常界面英文：设置、菜单、slash、欢迎语。只译我们用得到的表面。
  Use when the user asks 收英文, 中文化, leftover English, or /tzworp-chinese-ui.
---

# tzworp-chinese-ui

产品原则在 `docs/TZWARP.md`：用得上的做成，用不上的不弄；硬编码中文，不做 i18n。

## 做

- 用户每天看得到的：设置页标题/说明、终端菜单、欢迎/zero state、slash 静态命令、Agent 提示、MCP 设置文案。
- 改设置页先读 `gui-settings-ui`：标题放 `PageType` 槽，不要header-only widget。
- `BindingDescription::new` 会 titlecase 英文。中文短句不要塞进去再当翻译。

## 不要

- 不要翻 Warp Drive / Cloud Agent / Handoff / 登录计费。这些在 `team_relay` 下应继续 `#[cfg(not(feature = "team_relay"))]` 或不可达。
- 不要引入 gettext / fluent / 语言切换。
- 不要为了「全绿」去改官方测试里的英文断言。

## 验收

打开会用到的设置页和终端菜单扫一遍。欢迎语在**新 tab** 和恢复的会话都看一下（恢复 tab 以前漏过英文）。
