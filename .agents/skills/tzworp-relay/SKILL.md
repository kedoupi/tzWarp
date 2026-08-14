---
name: tzworp-relay
description: >
  tzWarp team_relay 智能体：中转站消息拼装、slash 拦截、确认执行回传、复读截停。
  Use when changing team_relay, /plan, slash in terminal, confirm-run, tool pairing,
  mid-station 400, or 小桃子 chat. Triggers: /tzworp-relay, 中转, 确认执行, tool call_id.
---

# tzworp-relay

常驻产品约束读 `docs/TZWARP.md`。本 skill 只覆盖改智能体代码时的做法。

代码在 `app/src/ai/team_relay/`。slash 在 `app/src/terminal/input/slash_commands/` 和 `slash_command_model.rs`。

## 中转合同

- 中转无状态。每条 `role=tool` 必须紧跟带匹配 `tool_calls[].id` 的 assistant。中间插 user 会 400：`No tool call found for function call output`。
- 只有 `CallMcpTool` 发 `role=tool`。`RunShellCommand` 结果用 `format_shell_command_result` 当 **assistant 文本**。
- `assemble_openai_messages`：有 MCP pending 时不要在 tool 对后面插用户原话。
- 只有 native shell 摘要、没有新 user 时，必须追加 `NATIVE_CONTINUATION_PROMPT`。不要用「终端上下文已经当 user 发过」把续写提示去重掉。
- 历史里的 `AgentOutput` 先 `collapse_repetition` 再回灌。
- 流式：用 `incremental_text_delta`（兼容累计快照）。`is_degenerate_tail` 为真就停，并跳过 `extract_shell_commands`。
- 已经在历史 bash 围栏里出现过的命令不要再弹确认卡。

## slash

- `team_relay` 下终端必须解析 slash（`slash_commands_available_in_terminal`）。
- 从终端执行 Plan/Compact/Orchestrate：发 `EnterAgentView { initial_prompt: 完整 "/plan …" }`，不要 `set_input_type(AI)`（locked Shell 会静默失败，命令进 PTY）。
- 已在智能体里：写 buffer 再 `submit_ai_query_with_routing`。
- 官方 planning 工具在 `team_relay` 下是关的。`/plan` 是带前缀的中转对话，不是 Oz planner。

## 改完至少跑

```bash
cargo test -p warp --lib --features team_relay -- ai::team_relay terminal::input::slash_commands::tests
```

手测：新对话 `/plan 这是什么项目呢` → 确认执行一条命令 → 「做个总结吧」不得复读标题。
