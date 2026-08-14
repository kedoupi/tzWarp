use std::collections::HashSet;

use super::*;

fn assistant_call(id: &str, name: &str) -> ChatMessage {
    ChatMessage {
        role: "assistant".into(),
        content: None,
        tool_calls: Some(vec![OpenAIToolCall {
            id: id.into(),
            type_: "function".into(),
            function: OpenAIFunctionCall {
                name: name.into(),
                arguments: "{}".into(),
            },
        }]),
        tool_call_id: None,
    }
}

fn tool_result(id: &str, content: &str) -> ChatMessage {
    ChatMessage {
        role: "tool".into(),
        content: Some(content.into()),
        tool_calls: None,
        tool_call_id: Some(id.into()),
    }
}

fn assemble(
    history: Vec<ChatMessage>,
    next_user: Option<&str>,
    pending: Vec<(&str, &str)>,
    native: Vec<&str>,
) -> Vec<ChatMessage> {
    assemble_openai_messages(
        "sys".into(),
        history,
        next_user.map(str::to_owned),
        pending
            .into_iter()
            .map(|(id, content)| (id.to_owned(), content.to_owned()))
            .collect(),
        native.into_iter().map(str::to_owned).collect(),
    )
}

fn roles(messages: &[ChatMessage]) -> Vec<&str> {
    messages.iter().map(|m| m.role.as_str()).collect()
}

fn every_tool_follows_matching_call(messages: &[ChatMessage]) -> bool {
    let mut open_ids: HashSet<&str> = HashSet::new();
    for message in messages {
        match message.role.as_str() {
            "assistant" => {
                open_ids = message
                    .tool_calls
                    .as_ref()
                    .map(|calls| calls.iter().map(|call| call.id.as_str()).collect())
                    .unwrap_or_default();
            }
            "tool" => {
                let Some(id) = message.tool_call_id.as_deref() else {
                    return false;
                };
                if !open_ids.contains(id) {
                    return false;
                }
            }
            _ => open_ids.clear(),
        }
    }
    true
}

#[test]
fn extract_single_line_bash_blocks() {
    let md = "先看一下：\n```bash\nls -la\n```\n然后\n```sh\necho hi\n```\n```python\nprint(1)\n```\n```bash\necho a\necho b\n```";
    assert_eq!(
        extract_shell_commands(md),
        vec!["ls -la".to_string(), "echo hi".to_string()]
    );
}

#[test]
fn sanitize_401_is_human() {
    let msg = sanitize_relay_error(401, "{\"error\":\"nope\"}");
    assert!(msg.contains("密钥"));
    assert!(msg.contains("401"));
}

#[test]
fn parse_activate_query() {
    let url = url::Url::parse("tzworp://activate?token=sk-abc").unwrap();
    assert_eq!(parse_activate_url(&url).as_deref(), Some("sk-abc"));
}

#[test]
fn resolve_model_auto() {
    assert_eq!(resolve_model_name("auto", None), DEFAULT_MODEL);
    assert_eq!(resolve_model_name("gpt-5.4", None), "gpt-5.4");
}

#[test]
fn openai_tool_name_is_stable() {
    let name = openai_tool_name("a1b2c3d4-ffff", "list-issues");
    assert_eq!(name, "a1b2c3d4__list_issues");
}

#[test]
fn chat_message_tool_fields_omit_when_empty() {
    let msg = ChatMessage::text("user", "hi");
    let json = serde_json::to_value(&msg).unwrap();
    assert_eq!(json["role"], "user");
    assert_eq!(json["content"], "hi");
    assert!(json.get("tool_calls").is_none());
    assert!(json.get("tool_call_id").is_none());
}

#[test]
fn assemble_first_turn_is_system_then_user() {
    let messages = assemble(Vec::new(), Some("/plan 审查项目"), Vec::new(), Vec::new());
    assert_eq!(roles(&messages), ["system", "user"]);
    assert_eq!(messages[1].content.as_deref(), Some("/plan 审查项目"));
    assert!(every_tool_follows_matching_call(&messages));
}

#[test]
fn assemble_does_not_insert_user_between_tool_call_and_result() {
    let history = vec![
        ChatMessage::text("user", "/plan 审查项目"),
        assistant_call("fc_abc", "a1b2c3d4__list_issues"),
    ];
    let messages = assemble(
        history,
        Some("/plan 审查项目"),
        vec![("fc_abc", "ok")],
        Vec::new(),
    );
    assert_eq!(roles(&messages), ["system", "user", "assistant", "tool"]);
    assert_eq!(messages[3].tool_call_id.as_deref(), Some("fc_abc"));
    assert_eq!(messages[2].tool_calls.as_ref().unwrap()[0].id, "fc_abc");
    assert!(every_tool_follows_matching_call(&messages));
}

#[test]
fn assemble_synthesizes_missing_assistant_tool_call() {
    let history = vec![ChatMessage::text("user", "/plan 审查项目")];
    let messages = assemble(
        history,
        None,
        vec![(
            "fc_d4c1cbca-9810-4a9e-99c0-ed73c73580d2",
            "tool output",
        )],
        Vec::new(),
    );
    assert_eq!(roles(&messages), ["system", "user", "assistant", "tool"]);
    let call = &messages[2].tool_calls.as_ref().unwrap()[0];
    assert_eq!(call.id, "fc_d4c1cbca-9810-4a9e-99c0-ed73c73580d2");
    assert_eq!(messages[3].tool_call_id.as_deref(), Some(call.id.as_str()));
    assert!(every_tool_follows_matching_call(&messages));
}

#[test]
fn assemble_repairs_orphaned_tool_already_in_history() {
    let history = vec![
        ChatMessage::text("user", "/plan 审查项目"),
        tool_result("fc_orphan", "stale output"),
    ];
    let messages = assemble(history, None, Vec::new(), Vec::new());
    assert_eq!(roles(&messages), ["system", "user", "assistant", "tool"]);
    assert_eq!(
        messages[2].tool_calls.as_ref().unwrap()[0].id,
        "fc_orphan"
    );
    assert!(every_tool_follows_matching_call(&messages));
}

#[test]
fn assemble_native_shell_summary_is_assistant_text_not_tool() {
    let history = vec![ChatMessage::text("user", "看一下目录")];
    let messages = assemble(
        history,
        None,
        Vec::new(),
        vec!["命令已执行（exit 0）\n```\nok\n```"],
    );
    assert_eq!(roles(&messages), ["system", "user", "assistant", "user"]);
    assert!(messages.iter().all(|m| m.role != "tool"));
    assert!(messages[2].content.as_deref().unwrap().contains("exit 0"));
    assert!(
        messages[3]
            .content
            .as_deref()
            .unwrap()
            .contains("继续回答")
    );
    assert!(every_tool_follows_matching_call(&messages));
}

#[test]
fn assemble_keeps_native_summary_after_mcp_pair() {
    let history = vec![
        ChatMessage::text("user", "/plan 审查项目"),
        assistant_call("fc_abc", "a1b2c3d4__list_issues"),
    ];
    let messages = assemble(
        history,
        Some("/plan 审查项目"),
        vec![("fc_abc", "ok")],
        vec!["命令已执行。"],
    );
    assert_eq!(
        roles(&messages),
        ["system", "user", "assistant", "tool", "assistant", "user"]
    );
    assert!(
        messages
            .last()
            .unwrap()
            .content
            .as_deref()
            .unwrap()
            .contains("继续回答")
    );
    assert!(every_tool_follows_matching_call(&messages));
}

#[test]
fn merge_consecutive_assistant_tool_calls_keeps_one_message() {
    let merged = merge_consecutive_assistant_tool_calls(vec![
        assistant_call("fc_1", "a__one"),
        assistant_call("fc_2", "a__two"),
    ]);
    assert_eq!(merged.len(), 1);
    let ids: Vec<_> = merged[0]
        .tool_calls
        .as_ref()
        .unwrap()
        .iter()
        .map(|c| c.id.as_str())
        .collect();
    assert_eq!(ids, ["fc_1", "fc_2"]);
}

#[test]
fn apply_tool_call_deltas_merges_empty_id_into_open_slot() {
    let mut acc = vec![OpenAIToolCall {
        id: String::new(),
        type_: "function".into(),
        function: OpenAIFunctionCall {
            name: "a__one".into(),
            arguments: String::new(),
        },
    }];
    let parsed = serde_json::from_str::<ChatCompletionResponse>(
        r#"{"choices":[{"message":{"tool_calls":[{"id":"fc_1","type":"function","function":{"name":"","arguments":"{}"}}]}}]}"#,
    )
    .unwrap();
    apply_tool_call_deltas(&mut acc, &parsed);
    assert_eq!(acc.len(), 1);
    assert_eq!(acc[0].id, "fc_1");
    assert_eq!(acc[0].function.name, "a__one");
    assert_eq!(acc[0].function.arguments, "{}");
}

#[test]
fn assemble_adds_continuation_after_duplicate_native_summary() {
    let history = vec![
        ChatMessage::text("user", "你做个总结吧"),
        ChatMessage::text("user", NATIVE_CONTINUATION_PROMPT),
        ChatMessage::text("assistant", "命令已执行（exit 0）\n```\nok\n```"),
    ];
    let messages = assemble(
        history,
        None,
        Vec::new(),
        vec!["命令已执行（exit 0）\n```\nok\n```"],
    );
    assert_eq!(messages.last().unwrap().role, "user");
    assert!(
        messages
            .last()
            .unwrap()
            .content
            .as_deref()
            .unwrap()
            .contains("继续回答")
    );
    assert!(every_tool_follows_matching_call(&messages));
}

#[test]
fn assemble_does_not_drop_new_user_that_appeared_earlier() {
    let history = vec![
        ChatMessage::text("user", "ls"),
        ChatMessage::text("assistant", "目录如下"),
    ];
    let messages = assemble(history, Some("ls"), Vec::new(), Vec::new());
    assert_eq!(roles(&messages), ["system", "user", "assistant", "user"]);
    assert_eq!(messages.last().unwrap().content.as_deref(), Some("ls"));
}

#[test]
fn incremental_text_delta_accepts_true_delta() {
    assert_eq!(
        incremental_text_delta("你好", "世界").as_deref(),
        Some("世界")
    );
}

#[test]
fn incremental_text_delta_strips_cumulative_snapshot() {
    assert_eq!(
        incremental_text_delta("你好", "你好世界").as_deref(),
        Some("世界")
    );
}

#[test]
fn incremental_text_delta_skips_duplicate_snapshot() {
    assert_eq!(incremental_text_delta("你好", "你好"), None);
}

#[test]
fn is_degenerate_tail_detects_title_loop() {
    assert!(is_degenerate_tail(&"工作流".repeat(20)));
    assert!(is_degenerate_tail(
        &"FDE 小红书内容生产工作流 ".repeat(8)
    ));
    assert!(!is_degenerate_tail(
        "下面是基于项目文档的总结。这是一个 FDE 小红书内容生产工作流。"
    ));
}

#[test]
fn collapse_repetition_truncates_title_loop() {
    let looped = format!(
        "基于文档核对后的总结：{}",
        "FDE 小红书内容生产工作流 ".repeat(12)
    );
    let out = collapse_repetition(&looped);
    assert!(out.chars().count() < looped.chars().count());
    assert!(out.contains("基于文档核对后的总结"));
    assert!(out.contains("FDE 小红书内容生产工作流"));
    assert!(!is_degenerate_tail(&out));
}

#[test]
fn collapse_repetition_keeps_normal_prose() {
    let text = "下面是基于项目文档的总结。这是一个 FDE 小红书内容生产工作流，覆盖选题到发布。";
    assert_eq!(collapse_repetition(text), text);
}

#[test]
fn known_shell_commands_come_from_history_fences() {
    let messages = vec![ChatMessage::text(
        "assistant",
        "建议执行命令：\n```bash\ncat docs/project-workflow.md\n```",
    )];
    let known = collect_known_shell_commands(&messages);
    assert!(known.contains("cat docs/project-workflow.md"));
}

#[test]
fn apply_tool_call_deltas_uses_stream_index() {
    let mut acc = Vec::new();
    let first = serde_json::from_str::<ChatCompletionResponse>(
        r#"{"choices":[{"delta":{"tool_calls":[{"index":0,"id":"fc_1","function":{"name":"a__one","arguments":""}}]}}]}"#,
    )
    .unwrap();
    let second = serde_json::from_str::<ChatCompletionResponse>(
        r#"{"choices":[{"delta":{"tool_calls":[{"index":0,"function":{"name":"","arguments":"{\"q\":1}"}}]}}]}"#,
    )
    .unwrap();
    apply_tool_call_deltas(&mut acc, &first);
    apply_tool_call_deltas(&mut acc, &second);
    assert_eq!(acc.len(), 1);
    assert_eq!(acc[0].id, "fc_1");
    assert_eq!(acc[0].function.name, "a__one");
    assert_eq!(acc[0].function.arguments, "{\"q\":1}");
}
