//! tzWarp Team Relay: 客户端直连团队 OpenAI 兼容中转站，不经 Warp Server。
//!
//! Base URL 默认锁定为 `https://tzai.kdp.cool/v1`，用户只需配置 API Key。

use std::collections::HashSet;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use futures::stream::Stream;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use warp_multi_agent_api as api;

use crate::server::server_api::AIApiError;

/// 团队中转站默认 Base URL（含 `/v1`）。
pub const DEFAULT_BASE_URL: &str = "https://tzai.kdp.cool/v1";

/// 小桃子申请账号 / 领取 API 密钥的页面。
pub const SIGNUP_URL: &str = "https://tzai.kdp.cool";

/// 设置/模型选择器中显示的名称。
pub const RELAY_DISPLAY_NAME: &str = "tzWarp 中转站";

/// 默认模型（中转站可用列表中的稳定项）。
pub const DEFAULT_MODEL: &str = "gpt-5.4-mini";

const SYSTEM_PROMPT: &str = "\
你是 tzWarp 终端里的智能体，直接帮用户处理当前工作目录里的事。

原则：
- 默认中文，简洁，不要复读同一个词、标题或句子
- 结合用户提供的当前目录、选中内容和最近命令输出作答
- 对话里已经有文件内容或命令输出时直接使用，不要再执行同一条命令
- 需要执行的 shell 命令单独放在 markdown 代码块中，语言标记用 bash 或 sh，每块一条、不要 $ 前缀
- 命令会先展示给用户，确认后才运行
- 若提供了 MCP 工具，优先用工具查真实数据，不要编造
- 不确定就说不确定，不要编造文件内容或命令输出
";

/// 确认执行后的 shell 结果只能以 assistant 文本回灌。若请求末尾停在这段
/// 文本上、后面没有新的 user，模型会接着文档标题续写并陷入复读。
const NATIVE_CONTINUATION_PROMPT: &str = "\
请根据刚才的命令输出继续回答用户。用简洁中文，不要复读同一个词或标题；\
文件内容已经在上面时不要再执行同一条命令。\
";

/// 解析实际 Base URL：环境变量可覆盖（仅用于调试）。
pub fn base_url() -> String {
    std::env::var("TEAM_RELAY_BASE_URL")
        .or_else(|_| std::env::var("TZAI_BASE_URL"))
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_BASE_URL.to_string())
}

/// Token 文件路径（小桃子分发 / 本地激活可用）。
/// 优先 `~/.tzworp/token`，其次应用数据目录下 `team_token`。
pub fn token_file_candidates() -> Vec<std::path::PathBuf> {
    let mut paths = Vec::new();
    if let Some(home) = dirs::home_dir() {
        paths.push(home.join(".tzworp").join("token"));
    }
    // warp_core data_dir 在运行时才稳；这里再补常见相对路径
    paths.push(std::path::PathBuf::from("team_token"));
    paths
}

/// 从磁盘读取小桃子/中转站 token（第一行非空文本）。
pub fn read_token_from_disk() -> Option<String> {
    for path in token_file_candidates() {
        if let Ok(content) = std::fs::read_to_string(&path) {
            let t = content
                .lines()
                .map(str::trim)
                .find(|l| !l.is_empty() && !l.starts_with('#'));
            if let Some(token) = t {
                log::info!("loaded team token from {}", path.display());
                return Some(token.to_string());
            }
        }
    }
    None
}

/// 把 token 写入 `~/.tzworp/token`，便于重装后仍可用。
pub fn write_token_to_disk(token: &str) -> Result<(), String> {
    let token = token.trim();
    if token.is_empty() {
        return Err("token 为空".into());
    }
    let home = dirs::home_dir().ok_or_else(|| "无法定位用户主目录".to_string())?;
    let dir = home.join(".tzworp");
    std::fs::create_dir_all(&dir).map_err(|e| format!("创建 ~/.tzworp 失败: {e}"))?;
    let path = dir.join("token");
    std::fs::write(&path, format!("{token}\n")).map_err(|e| format!("写入 token 失败: {e}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

/// 解析 API Key：环境变量 → 磁盘 token → 安全存储传入值。
pub fn resolve_api_key(stored: Option<&str>) -> Option<String> {
    for var in [
        "TEAM_RELAY_API_KEY",
        "TZAI_API_KEY",
        "XIAOTAOZI_TOKEN",
        "TEAM_TOKEN",
    ] {
        if let Ok(v) = std::env::var(var) {
            let t = v.trim();
            if !t.is_empty() {
                return Some(t.to_string());
            }
        }
    }
    if let Some(t) = read_token_from_disk() {
        return Some(t);
    }
    stored
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned)
}

/// 激活小桃子/中转站 token：只写入 `~/.tzworp/token`。
pub fn activate_token(token: &str, _ctx: &mut warpui::AppContext) -> Result<(), String> {
    let token = token.trim();
    if token.is_empty() {
        return Err("激活失败：token 为空".into());
    }
    write_token_to_disk(token)
}

/// 解析 `tzworp://activate?token=...` 或 `tzworp://activate/<token>`。
pub fn parse_activate_url(url: &url::Url) -> Option<String> {
    if url.scheme() != "tzworp" {
        return None;
    }
    let host = url.host_str().unwrap_or("");
    if host != "activate" && host != "auth" {
        // 也支持 tzworp://token?value=
        if host != "token" {
            return None;
        }
    }
    // query: token= / key= / api_key=
    for (k, v) in url.query_pairs() {
        if matches!(k.as_ref(), "token" | "key" | "api_key" | "apikey") {
            let t = v.trim();
            if !t.is_empty() {
                return Some(t.to_string());
            }
        }
    }
    // path remainder: /sk-xxx
    let path = url.path().trim_matches('/');
    if !path.is_empty() && path != "activate" {
        return Some(path.to_string());
    }
    None
}

#[derive(Debug, Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatMessage>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<OpenAITool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<OpenAIToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
}

impl ChatMessage {
    fn text(role: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: role.into(),
            content: Some(content.into()),
            tool_calls: None,
            tool_call_id: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OpenAITool {
    #[serde(rename = "type")]
    type_: String,
    function: OpenAIFunctionDef,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OpenAIFunctionDef {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    parameters: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OpenAIToolCall {
    id: String,
    #[serde(rename = "type")]
    type_: String,
    function: OpenAIFunctionCall,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct OpenAIFunctionCall {
    #[serde(default)]
    name: String,
    #[serde(default)]
    arguments: String,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    #[serde(default)]
    message: Option<AssistantMessage>,
    #[serde(default)]
    delta: Option<Delta>,
}

#[derive(Debug, Deserialize)]
struct AssistantMessage {
    content: Option<String>,
    #[serde(default)]
    tool_calls: Option<Vec<OpenAIToolCall>>,
}

#[derive(Debug, Deserialize)]
struct Delta {
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    tool_calls: Option<Vec<StreamedToolCallDelta>>,
}

#[derive(Debug, Deserialize)]
struct StreamedToolCallDelta {
    #[serde(default)]
    index: usize,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    function: Option<OpenAIFunctionCall>,
}

#[derive(Debug, Deserialize)]
struct ModelsResponse {
    data: Vec<ModelInfo>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub owned_by: Option<String>,
}

impl ModelInfo {
    pub fn label(&self) -> String {
        self.display_name
            .as_deref()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or(&self.id)
            .to_string()
    }
}

/// 从中转站拉取模型列表。
pub async fn fetch_models(api_key: &str) -> Result<Vec<ModelInfo>, String> {
    let url = format!("{}/models", base_url().trim_end_matches('/'));
    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .header("Authorization", format!("Bearer {api_key}"))
        .send()
        .await
        .map_err(|e| format!("获取模型列表失败: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("获取模型列表错误 ({status}): {body}"));
    }

    let parsed: ModelsResponse = resp
        .json()
        .await
        .map_err(|e| format!("解析模型列表失败: {e}"))?;

    let mut models = parsed.data;
    models.retain(|m| {
        let id = m.id.to_ascii_lowercase();
        !id.contains("image") && !id.contains("dall-e") && !id.contains("tts")
    });
    models.sort_by(|a, b| a.id.cmp(&b.id));
    if models.is_empty() {
        models.push(ModelInfo {
            id: DEFAULT_MODEL.into(),
            display_name: Some("GPT-5.4 Mini".into()),
            owned_by: None,
        });
    }
    Ok(models)
}

fn now_ts() -> prost_types::Timestamp {
    prost_types::Timestamp {
        seconds: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64,
        nanos: 0,
    }
}

fn make_init(conversation_id: &str, request_id: &str) -> api::ResponseEvent {
    api::ResponseEvent {
        r#type: Some(api::response_event::Type::Init(
            api::response_event::StreamInit {
                conversation_id: conversation_id.to_string(),
                request_id: request_id.to_string(),
                run_id: Uuid::new_v4().to_string(),
            },
        )),
    }
}

fn make_agent_message(
    task_id: &str,
    request_id: &str,
    message_id: &str,
    text: &str,
) -> api::Message {
    api::Message {
        id: message_id.to_string(),
        task_id: task_id.to_string(),
        request_id: request_id.to_string(),
        timestamp: Some(now_ts()),
        server_message_data: String::new(),
        citations: vec![],
        fetched_memories: vec![],
        message: Some(api::message::Message::AgentOutput(
            api::message::AgentOutput {
                text: text.to_string(),
            },
        )),
    }
}

fn make_agent_output_event(
    task_id: &str,
    request_id: &str,
    message_id: &str,
    text: &str,
    task_exists: bool,
) -> api::ResponseEvent {
    let agent_message = make_agent_message(task_id, request_id, message_id, text);

    let action = if task_exists {
        api::client_action::Action::AddMessagesToTask(api::client_action::AddMessagesToTask {
            task_id: task_id.to_string(),
            messages: vec![agent_message],
        })
    } else {
        api::client_action::Action::CreateTask(api::client_action::CreateTask {
            task: Some(api::Task {
                id: task_id.to_string(),
                description: "tzWarp AI 回复".into(),
                dependencies: None,
                messages: vec![agent_message],
                summary: String::new(),
                server_data: String::new(),
            }),
        })
    };

    let client_action = api::ClientAction {
        action: Some(action),
    };

    api::ResponseEvent {
        r#type: Some(api::response_event::Type::ClientActions(
            api::response_event::ClientActions {
                actions: vec![client_action],
            },
        )),
    }
}

fn make_finished() -> api::ResponseEvent {
    api::ResponseEvent {
        r#type: Some(api::response_event::Type::Finished(
            api::response_event::StreamFinished {
                token_usage: vec![],
                should_refresh_model_config: false,
                request_cost: None,
                conversation_usage_metadata: None,
                reason: Some(api::response_event::stream_finished::Reason::Done(
                    api::response_event::stream_finished::Done {},
                )),
            },
        )),
    }
}

/// 从 Warp multi-agent Request 提取用户当前输入。
pub fn extract_user_message(request: &api::Request) -> Option<String> {
    use api::request::input::Type;
    use api::request::input::user_inputs::user_input::Input as UserInputType;

    let input = request.input.as_ref()?;
    match &input.r#type {
        Some(Type::UserInputs(user_inputs)) => {
            for ui in &user_inputs.inputs {
                match &ui.input {
                    Some(UserInputType::UserQuery(query)) if !query.query.trim().is_empty() => {
                        return Some(query.query.clone());
                    }
                    Some(UserInputType::CliAgentUserQuery(cli)) => {
                        if let Some(q) = cli.user_query.as_ref() {
                            if !q.query.trim().is_empty() {
                                return Some(q.query.clone());
                            }
                        }
                    }
                    _ => {}
                }
            }
            None
        }
        Some(Type::QueryWithCannedResponse(q)) => Some(q.query.clone()),
        _ => None,
    }
}

/// 当前请求是否带着任意工具结果（MCP 或确认执行后的 shell）。
pub fn has_tool_results(request: &api::Request) -> bool {
    for_each_input_tool_result(request).next().is_some()
}

fn for_each_input_tool_result(
    request: &api::Request,
) -> impl Iterator<Item = &api::request::input::ToolCallResult> {
    use api::request::input::Type;
    use api::request::input::user_inputs::user_input::Input as UserInputType;

    request
        .input
        .as_ref()
        .and_then(|input| match &input.r#type {
            Some(Type::UserInputs(user_inputs)) => Some(user_inputs.inputs.as_slice()),
            _ => None,
        })
        .into_iter()
        .flatten()
        .filter_map(|ui| match &ui.input {
            Some(UserInputType::ToolCallResult(result)) => Some(result),
            _ => None,
        })
}

/// 只有 MCP 结果走 OpenAI `role=tool`。确认执行后的 shell / 其它原生结果不能当 function output。
fn collect_input_mcp_tool_results(request: &api::Request) -> Vec<(String, String)> {
    use api::request::input::tool_call_result::Result as ToolResult;
    for_each_input_tool_result(request)
        .filter_map(|result| match result.result.as_ref() {
            Some(ToolResult::CallMcpTool(mcp)) => {
                Some((result.tool_call_id.clone(), format_mcp_tool_result(mcp)))
            }
            _ => None,
        })
        .collect()
}

fn collect_input_native_tool_summaries(request: &api::Request) -> Vec<String> {
    use api::request::input::tool_call_result::Result as ToolResult;
    for_each_input_tool_result(request)
        .filter_map(|result| match result.result.as_ref() {
            Some(ToolResult::CallMcpTool(_)) => None,
            Some(ToolResult::RunShellCommand(shell)) => Some(format_shell_command_result(shell)),
            Some(_) => Some("工具已执行。".into()),
            None => Some("（无结果）".into()),
        })
        .collect()
}

#[allow(deprecated)]
fn format_shell_command_result(result: &api::RunShellCommandResult) -> String {
    use api::run_shell_command_result::Result as ShellRes;
    match result.result.as_ref() {
        Some(ShellRes::CommandFinished(finished)) => {
            let output = finished.output.trim();
            let clipped = if output.chars().count() > 4000 {
                format!("{}…", output.chars().take(4000).collect::<String>())
            } else {
                output.to_string()
            };
            if clipped.is_empty() {
                format!("命令已执行（exit {}）。", finished.exit_code)
            } else {
                format!(
                    "命令已执行（exit {}）\n```\n{clipped}\n```",
                    finished.exit_code
                )
            }
        }
        Some(ShellRes::PermissionDenied(_)) => "用户拒绝了该命令。".into(),
        Some(ShellRes::LongRunningCommandSnapshot(_)) => "命令仍在运行。".into(),
        None if !result.output.trim().is_empty() => {
            format!("命令输出：\n{}", result.output.trim())
        }
        None => "命令已执行。".into(),
    }
}

fn format_mcp_tool_result(mcp: &api::CallMcpToolResult) -> String {
    use api::call_mcp_tool_result::Result as McpRes;
    match mcp.result.as_ref() {
        Some(McpRes::Success(success)) => {
            let mut parts = Vec::new();
            for item in &success.results {
                if let Some(api::call_mcp_tool_result::success::result::Result::Text(text)) =
                    &item.result
                {
                    parts.push(text.text.clone());
                }
            }
            if parts.is_empty() {
                "ok".into()
            } else {
                parts.join("\n")
            }
        }
        Some(McpRes::Error(error)) => format!("错误: {}", error.message),
        None => "（已取消）".into(),
    }
}

/// 从 task 上下文还原 user + assistant + 工具历史。
fn extract_history_messages(request: &api::Request) -> Vec<ChatMessage> {
    let mut out = Vec::new();
    let Some(task_ctx) = request.task_context.as_ref() else {
        return out;
    };
    for task in &task_ctx.tasks {
        for msg in &task.messages {
            let Some(inner) = msg.message.as_ref() else {
                continue;
            };
            match inner {
                api::message::Message::UserQuery(q) if !q.query.trim().is_empty() => {
                    out.push(ChatMessage::text("user", q.query.clone()));
                }
                api::message::Message::AgentOutput(o) if !o.text.trim().is_empty() => {
                    out.push(ChatMessage::text("assistant", collapse_repetition(&o.text)));
                }
                api::message::Message::ToolCall(call) => match &call.tool {
                    Some(api::message::tool_call::Tool::RunShellCommand(cmd))
                        if !cmd.command.trim().is_empty() =>
                    {
                        out.push(ChatMessage::text(
                            "assistant",
                            format!("建议执行命令：\n```bash\n{}\n```", cmd.command),
                        ));
                    }
                    Some(api::message::tool_call::Tool::CallMcpTool(mcp)) => {
                        let name = openai_tool_name(&mcp.server_id, &mcp.name);
                        let arguments = mcp
                            .args
                            .as_ref()
                            .map(prost_struct_to_json)
                            .and_then(|v| serde_json::to_string(&v).ok())
                            .unwrap_or_else(|| "{}".into());
                        out.push(ChatMessage {
                            role: "assistant".into(),
                            content: None,
                            tool_calls: Some(vec![OpenAIToolCall {
                                id: call.tool_call_id.clone(),
                                type_: "function".into(),
                                function: OpenAIFunctionCall { name, arguments },
                            }]),
                            tool_call_id: None,
                        });
                    }
                    _ => {}
                },
                api::message::Message::ToolCallResult(result) => {
                    match result.result.as_ref() {
                        Some(api::message::tool_call_result::Result::CallMcpTool(mcp)) => {
                            out.push(ChatMessage {
                                role: "tool".into(),
                                content: Some(format_mcp_tool_result(mcp)),
                                tool_calls: None,
                                tool_call_id: Some(result.tool_call_id.clone()),
                            });
                        }
                        Some(api::message::tool_call_result::Result::RunShellCommand(shell)) => {
                            out.push(ChatMessage::text(
                                "assistant",
                                format_shell_command_result(shell),
                            ));
                        }
                        Some(_) => {
                            out.push(ChatMessage::text("assistant", "工具已执行。"));
                        }
                        None => {}
                    }
                }
                _ => {}
            }
        }
    }
    merge_consecutive_assistant_tool_calls(out)
}

fn merge_consecutive_assistant_tool_calls(messages: Vec<ChatMessage>) -> Vec<ChatMessage> {
    let mut out = Vec::with_capacity(messages.len());
    for msg in messages {
        let can_merge = msg.role == "assistant"
            && msg.content.is_none()
            && msg
                .tool_calls
                .as_ref()
                .is_some_and(|calls| !calls.is_empty())
            && out.last().is_some_and(|last: &ChatMessage| {
                last.role == "assistant"
                    && last.content.is_none()
                    && last
                        .tool_calls
                        .as_ref()
                        .is_some_and(|calls| !calls.is_empty())
            });
        if can_merge {
            if let Some(last) = out.last_mut()
                && let Some(calls) = msg.tool_calls
            {
                last.tool_calls.get_or_insert_with(Vec::new).extend(calls);
            }
        } else {
            out.push(msg);
        }
    }
    out
}

fn assistant_tool_call_ids(messages: &[ChatMessage]) -> HashSet<String> {
    messages
        .iter()
        .filter_map(|m| m.tool_calls.as_ref())
        .flatten()
        .map(|c| c.id.clone())
        .collect()
}

fn name_for_tool_call_id(messages: &[ChatMessage], tool_call_id: &str) -> String {
    messages
        .iter()
        .filter_map(|m| m.tool_calls.as_ref())
        .flatten()
        .find(|c| c.id == tool_call_id)
        .map(|c| c.function.name.clone())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "call_mcp_tool".into())
}

/// OpenAI 兼容接口要求 `role=tool` 必须紧跟带有对应 `tool_calls[].id` 的 assistant。
/// 中转站无状态，漏掉这次 function call 或在中间插入 user，就会 400：
/// `No tool call found for function call output with call_id …`
fn assemble_openai_messages(
    system: String,
    history: Vec<ChatMessage>,
    next_user: Option<String>,
    pending_tool_results: Vec<(String, String)>,
    native_summaries: Vec<String>,
) -> Vec<ChatMessage> {
    let has_native = native_summaries.iter().any(|summary| !summary.is_empty());
    let mut messages = vec![ChatMessage::text("system", system)];
    messages.extend(merge_consecutive_assistant_tool_calls(history));

    let pending: Vec<(String, String)> = pending_tool_results
        .into_iter()
        .filter(|(id, _)| {
            !messages
                .iter()
                .any(|m| m.tool_call_id.as_deref() == Some(id.as_str()))
        })
        .collect();

    if !pending.is_empty() {
        let known = assistant_tool_call_ids(&messages);
        let missing: Vec<OpenAIToolCall> = pending
            .iter()
            .filter(|(id, _)| !known.contains(id))
            .map(|(id, _)| OpenAIToolCall {
                id: id.clone(),
                type_: "function".into(),
                function: OpenAIFunctionCall {
                    name: name_for_tool_call_id(&messages, id),
                    arguments: "{}".into(),
                },
            })
            .collect();
        if !missing.is_empty() {
            log::warn!(
                "team_relay: synthesizing {} missing assistant tool_calls before tool results",
                missing.len()
            );
            messages.push(ChatMessage {
                role: "assistant".into(),
                content: None,
                tool_calls: Some(missing),
                tool_call_id: None,
            });
        }
        for (tool_call_id, content) in pending {
            messages.push(ChatMessage {
                role: "tool".into(),
                content: Some(content),
                tool_calls: None,
                tool_call_id: Some(tool_call_id),
            });
        }
        append_native_summaries(&mut messages, native_summaries);
        // MCP 的 tool 对后面不要插 user。只有额外回了 shell 文本时才需要续写提示。
        if has_native {
            push_user_if_not_last(&mut messages, NATIVE_CONTINUATION_PROMPT);
        }
        return repair_tool_pairing(messages);
    }

    append_native_summaries(&mut messages, native_summaries);

    if let Some(user) = next_user.filter(|s| !s.is_empty()) {
        push_user_if_not_last(&mut messages, user);
    } else if has_native {
        push_user_if_not_last(&mut messages, NATIVE_CONTINUATION_PROMPT);
    }

    repair_tool_pairing(messages)
}

fn push_user_if_not_last(messages: &mut Vec<ChatMessage>, user: impl Into<String>) {
    let user = user.into();
    let last_is_same = messages.last().is_some_and(|message| {
        message.role == "user" && message.content.as_deref() == Some(user.as_str())
    });
    if !last_is_same {
        messages.push(ChatMessage::text("user", user));
    }
}

fn append_native_summaries(messages: &mut Vec<ChatMessage>, summaries: Vec<String>) {
    for summary in summaries {
        if summary.is_empty() {
            continue;
        }
        if messages
            .iter()
            .any(|m| m.role == "assistant" && m.content.as_deref() == Some(summary.as_str()))
        {
            continue;
        }
        messages.push(ChatMessage::text("assistant", summary));
    }
}

/// 保证每条 `role=tool` 都紧跟带有对应 `tool_calls[].id` 的 assistant。
fn repair_tool_pairing(messages: Vec<ChatMessage>) -> Vec<ChatMessage> {
    let mut out = Vec::with_capacity(messages.len() + 2);
    let mut index = 0;
    while index < messages.len() {
        if messages[index].role != "tool" {
            out.push(messages[index].clone());
            index += 1;
            continue;
        }
        let start = index;
        while index < messages.len() && messages[index].role == "tool" {
            index += 1;
        }
        let tools = &messages[start..index];
        let preceding_ids: HashSet<String> = out
            .last()
            .and_then(|last| {
                (last.role == "assistant")
                    .then_some(last.tool_calls.as_ref())
                    .flatten()
            })
            .map(|calls| calls.iter().map(|call| call.id.clone()).collect())
            .unwrap_or_default();
        let missing: Vec<OpenAIToolCall> = tools
            .iter()
            .filter_map(|tool| tool.tool_call_id.as_ref())
            .filter(|id| !preceding_ids.contains(*id))
            .map(|id| OpenAIToolCall {
                id: id.clone(),
                type_: "function".into(),
                function: OpenAIFunctionCall {
                    name: name_for_tool_call_id(&out, id),
                    arguments: "{}".into(),
                },
            })
            .collect();
        if !missing.is_empty() {
            if let Some(last) = out.last_mut()
                && last.role == "assistant"
                && last.content.is_none()
                && last.tool_calls.is_some()
            {
                last.tool_calls
                    .get_or_insert_with(Vec::new)
                    .extend(missing);
            } else {
                out.push(ChatMessage {
                    role: "assistant".into(),
                    content: None,
                    tool_calls: Some(missing),
                    tool_call_id: None,
                });
            }
        }
        out.extend(tools.iter().cloned());
    }
    out
}

fn format_project_rules(request: &api::Request) -> String {
    let rules_enabled = request
        .settings
        .as_ref()
        .is_some_and(|s| s.rules_enabled);
    if !rules_enabled {
        return String::new();
    }
    let Some(ctx) = request.input.as_ref().and_then(|i| i.context.as_ref()) else {
        return String::new();
    };
    let mut blocks = Vec::new();
    for rules in &ctx.project_rules {
        for file in &rules.active_rule_files {
            let path = if file.file_path.is_empty() {
                "规则".to_string()
            } else {
                file.file_path.clone()
            };
            let content = file.content.trim();
            if content.is_empty() {
                continue;
            }
            let clipped = if content.chars().count() > 6000 {
                format!("{}…", content.chars().take(6000).collect::<String>())
            } else {
                content.to_string()
            };
            blocks.push(format!("### {path}\n{clipped}"));
        }
    }
    if blocks.is_empty() {
        String::new()
    } else {
        format!("[本地规则]\n{}\n", blocks.join("\n\n"))
    }
}

fn openai_tool_name(server_id: &str, tool_name: &str) -> String {
    let mut prefix: String = server_id
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .take(8)
        .collect();
    if prefix.is_empty() {
        prefix = "mcp".into();
    }
    let tool: String = tool_name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    format!("{prefix}__{tool}")
}

fn parse_openai_tool_name<'a>(
    name: &str,
    servers: &'a [api::request::mcp_context::McpServer],
) -> Option<(&'a str, &'a str)> {
    if let Some((prefix, tool)) = name.split_once("__") {
        for server in servers {
            if openai_tool_name(&server.id, tool) == name
                || server.id.chars().filter(|c| c.is_ascii_alphanumeric()).take(8).eq(prefix.chars())
            {
                if let Some(found) = server.tools.iter().find(|t| {
                    t.name == tool
                        || openai_tool_name(&server.id, &t.name) == name
                }) {
                    return Some((server.id.as_str(), found.name.as_str()));
                }
            }
        }
    }
    for server in servers {
        if let Some(found) = server.tools.iter().find(|t| t.name == name) {
            return Some((server.id.as_str(), found.name.as_str()));
        }
    }
    None
}

#[allow(deprecated)]
fn mcp_tools_from_request(request: &api::Request) -> (Vec<OpenAITool>, Vec<api::request::mcp_context::McpServer>) {
    let Some(ctx) = request.mcp_context.as_ref() else {
        return (Vec::new(), Vec::new());
    };
    let mut servers = ctx.servers.clone();
    if servers.is_empty() && (!ctx.tools.is_empty()) {
        servers.push(api::request::mcp_context::McpServer {
            id: String::new(),
            name: "mcp".into(),
            description: String::new(),
            resources: Vec::new(),
            tools: ctx.tools.clone(),
        });
    }
    let mut tools = Vec::new();
    for server in &servers {
        for tool in &server.tools {
            let parameters = tool
                .input_schema
                .as_ref()
                .map(prost_struct_to_json)
                .unwrap_or_else(|| serde_json::json!({"type": "object", "properties": {}}));
            tools.push(OpenAITool {
                type_: "function".into(),
                function: OpenAIFunctionDef {
                    name: openai_tool_name(&server.id, &tool.name),
                    description: if tool.description.is_empty() {
                        None
                    } else {
                        Some(tool.description.clone())
                    },
                    parameters: Some(parameters),
                },
            });
        }
    }
    (tools, servers)
}

fn prost_value_to_json(value: &prost_types::Value) -> serde_json::Value {
    use prost_types::value::Kind;
    match value.kind.as_ref() {
        Some(Kind::NullValue(_)) | None => serde_json::Value::Null,
        Some(Kind::BoolValue(v)) => serde_json::Value::Bool(*v),
        Some(Kind::NumberValue(v)) => serde_json::Number::from_f64(*v)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        Some(Kind::StringValue(v)) => serde_json::Value::String(v.clone()),
        Some(Kind::ListValue(list)) => {
            serde_json::Value::Array(list.values.iter().map(prost_value_to_json).collect())
        }
        Some(Kind::StructValue(s)) => prost_struct_to_json(s),
    }
}

fn prost_struct_to_json(s: &prost_types::Struct) -> serde_json::Value {
    let map = s
        .fields
        .iter()
        .map(|(k, v)| (k.clone(), prost_value_to_json(v)))
        .collect();
    serde_json::Value::Object(map)
}

fn json_to_prost_struct(value: &serde_json::Value) -> Option<prost_types::Struct> {
    let obj = value.as_object()?;
    let mut fields = std::collections::BTreeMap::new();
    for (k, v) in obj {
        fields.insert(k.clone(), json_to_prost_value(v));
    }
    Some(prost_types::Struct { fields })
}

fn json_to_prost_value(value: &serde_json::Value) -> prost_types::Value {
    use prost_types::value::Kind;
    let kind = match value {
        serde_json::Value::Null => Kind::NullValue(0),
        serde_json::Value::Bool(v) => Kind::BoolValue(*v),
        serde_json::Value::Number(n) => Kind::NumberValue(n.as_f64().unwrap_or(0.0)),
        serde_json::Value::String(s) => Kind::StringValue(s.clone()),
        serde_json::Value::Array(a) => Kind::ListValue(prost_types::ListValue {
            values: a.iter().map(json_to_prost_value).collect(),
        }),
        serde_json::Value::Object(o) => Kind::StructValue(prost_types::Struct {
            fields: o
                .iter()
                .map(|(k, v)| (k.clone(), json_to_prost_value(v)))
                .collect(),
        }),
    };
    prost_types::Value { kind: Some(kind) }
}

fn make_call_mcp_tool(
    task_id: &str,
    request_id: &str,
    tool_call_id: &str,
    server_id: &str,
    name: &str,
    args: Option<prost_types::Struct>,
) -> api::ResponseEvent {
    let message = api::Message {
        id: Uuid::new_v4().to_string(),
        task_id: task_id.to_string(),
        request_id: request_id.to_string(),
        timestamp: Some(now_ts()),
        server_message_data: String::new(),
        citations: vec![],
        fetched_memories: vec![],
        message: Some(api::message::Message::ToolCall(api::message::ToolCall {
            tool_call_id: tool_call_id.to_string(),
            tool: Some(api::message::tool_call::Tool::CallMcpTool(
                api::message::tool_call::CallMcpTool {
                    server_id: server_id.to_string(),
                    name: name.to_string(),
                    args,
                },
            )),
        })),
    };
    make_actions(vec![api::client_action::Action::AddMessagesToTask(
        api::client_action::AddMessagesToTask {
            task_id: task_id.to_string(),
            messages: vec![message],
        },
    )])
}

fn format_terminal_context(request: &api::Request) -> String {
    let mut lines = Vec::new();
    let ctx = request.input.as_ref().and_then(|input| input.context.as_ref());
    if let Some(ctx) = ctx {
        if let Some(dir) = ctx.directory.as_ref()
            && !dir.pwd.trim().is_empty()
        {
            lines.push(format!("当前目录: {}", dir.pwd));
        }
        if let Some(os) = ctx.operating_system.as_ref()
            && !os.platform.trim().is_empty()
        {
            lines.push(format!("系统: {}", os.platform));
        }
        if let Some(shell) = ctx.shell.as_ref()
            && !shell.name.trim().is_empty()
        {
            lines.push(format!("Shell: {}", shell.name));
        }
        if let Some(git) = ctx.git.as_ref() {
            let branch = if !git.branch.trim().is_empty() {
                git.branch.as_str()
            } else {
                git.head.as_str()
            };
            if !branch.trim().is_empty() {
                lines.push(format!("Git: {branch}"));
            }
        }
        for selected in ctx.selected_text.iter().take(3) {
            let text = selected.text.trim();
            if !text.is_empty() {
                let clipped = if text.len() > 2000 {
                    format!("{}…", &text[..2000])
                } else {
                    text.to_string()
                };
                lines.push(format!("选中内容:\n{clipped}"));
            }
        }
    }
    if lines.is_empty() {
        String::new()
    } else {
        format!("[终端上下文]\n{}\n", lines.join("\n"))
    }
}

fn collect_known_shell_commands(messages: &[ChatMessage]) -> HashSet<String> {
    messages
        .iter()
        .filter_map(|message| message.content.as_deref())
        .flat_map(extract_shell_commands)
        .collect()
}

/// 中转站有的实现把 `message.content` 当累计全文、有的只给增量。
/// 已出现过的前缀或整段快照不要再拼一次。
fn incremental_text_delta(accumulated: &str, incoming: &str) -> Option<String> {
    if incoming.is_empty() {
        return None;
    }
    if incoming == accumulated {
        return None;
    }
    if !accumulated.is_empty() && incoming.starts_with(accumulated) {
        return Some(incoming[accumulated.len()..].to_string());
    }
    Some(incoming.to_string())
}

fn repetition_min_count(unit_len: usize) -> usize {
    match unit_len {
        1 => 20,
        2 => 12,
        3..=8 => 8,
        _ => 5,
    }
}

/// 文本末尾是否已经在空转同一个词/短句。
fn is_degenerate_tail(text: &str) -> bool {
    let chars: Vec<char> = text.chars().collect();
    let start = chars.len().saturating_sub(512);
    find_degenerate_repeat(&chars[start..]).is_some()
}

fn find_degenerate_repeat(chars: &[char]) -> Option<(usize, usize, usize)> {
    let n = chars.len();
    if n < 16 {
        return None;
    }
    let max_unit = 64.min(n / 4);
    let mut best: Option<(usize, usize, usize)> = None;
    for unit_len in 1..=max_unit {
        let unit = &chars[n - unit_len..n];
        let mut count = 1;
        let mut pos = n - unit_len;
        while pos >= unit_len && &chars[pos - unit_len..pos] == unit {
            count += 1;
            pos -= unit_len;
        }
        if count < repetition_min_count(unit_len) {
            continue;
        }
        let start = n - count * unit_len;
        let span = count * unit_len;
        if best.is_none_or(|(_, _, best_span)| best_span < span) {
            best = Some((start, unit_len, span));
        }
    }
    best
}

fn repeat_run_at(chars: &[char], start: usize) -> Option<(usize, usize)> {
    let remain = chars.len().saturating_sub(start);
    if remain < 16 {
        return None;
    }
    let max_unit = 64.min(remain / 4);
    let mut best: Option<(usize, usize)> = None;
    for unit_len in 1..=max_unit {
        let unit = &chars[start..start + unit_len];
        let mut count = 1;
        let mut pos = start + unit_len;
        while pos + unit_len <= chars.len() && &chars[pos..pos + unit_len] == unit {
            count += 1;
            pos += unit_len;
        }
        if count < repetition_min_count(unit_len) {
            continue;
        }
        let span = unit_len * count;
        if best.is_none_or(|(best_unit, best_count)| best_unit * best_count < span) {
            best = Some((unit_len, count));
        }
    }
    best
}

/// 把已经空转的词/短句收成两遍，避免下一轮把整段复读再喂回中转站。
fn collapse_repetition(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() < 16 {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len());
    let mut index = 0;
    let mut collapsed = false;
    while index < chars.len() {
        if let Some((unit_len, count)) = repeat_run_at(&chars, index) {
            out.extend(chars[index..index + unit_len * 2].iter());
            if count > 2 {
                out.push('…');
                collapsed = true;
            }
            index += unit_len * count;
        } else {
            out.push(chars[index]);
            index += 1;
        }
    }
    if collapsed { out } else { text.to_string() }
}

fn extract_shell_commands(markdown: &str) -> Vec<String> {
    let mut commands = Vec::new();
    let mut rest = markdown;
    while let Some(start) = rest.find("```") {
        rest = &rest[start + 3..];
        let Some(nl) = rest.find('\n') else {
            break;
        };
        let lang = rest[..nl].trim().to_ascii_lowercase();
        rest = &rest[nl + 1..];
        let Some(end) = rest.find("```") else {
            break;
        };
        let body = rest[..end].trim();
        rest = &rest[end + 3..];
        let is_shell = lang.is_empty()
            || matches!(
                lang.as_str(),
                "bash" | "sh" | "zsh" | "shell" | "console" | "terminal"
            );
        if !is_shell || body.is_empty() || body.contains('\n') {
            continue;
        }
        let cmd = body.trim_start_matches('$').trim();
        if !cmd.is_empty() {
            commands.push(cmd.to_string());
        }
    }
    commands
}

fn sanitize_relay_error(status: u16, body: &str) -> String {
    let hint = match status {
        401 | 403 => "密钥无效或没有权限，请在「设置 → 智能体」检查小桃子 API 密钥。",
        404 => "中转站接口不存在，请确认 TEAM_RELAY_BASE_URL。",
        429 => "请求过于频繁，请稍后再试。",
        500..=599 => "中转站暂时不可用，请稍后再试。",
        _ => "中转站返回错误。",
    };
    let snippet: String = body.chars().filter(|c| !c.is_control()).take(120).collect();
    if snippet.is_empty() {
        format!("{hint} (HTTP {status})")
    } else {
        format!("{hint} (HTTP {status}: {snippet})")
    }
}

fn make_actions(actions: Vec<api::client_action::Action>) -> api::ResponseEvent {
    api::ResponseEvent {
        r#type: Some(api::response_event::Type::ClientActions(
            api::response_event::ClientActions {
                actions: actions
                    .into_iter()
                    .map(|action| api::ClientAction {
                        action: Some(action),
                    })
                    .collect(),
            },
        )),
    }
}

fn make_append_delta(
    task_id: &str,
    request_id: &str,
    message_id: &str,
    delta: &str,
) -> api::ResponseEvent {
    make_actions(vec![
        api::client_action::Action::AppendToMessageContent(
            api::client_action::AppendToMessageContent {
                task_id: task_id.to_string(),
                message: Some(make_agent_message(task_id, request_id, message_id, delta)),
                mask: Some(prost_types::FieldMask {
                    paths: vec!["agent_output.text".to_string()],
                }),
            },
        ),
    ])
}

fn make_run_shell_command(task_id: &str, request_id: &str, command: &str) -> api::ResponseEvent {
    let message = api::Message {
        id: Uuid::new_v4().to_string(),
        task_id: task_id.to_string(),
        request_id: request_id.to_string(),
        timestamp: Some(now_ts()),
        server_message_data: String::new(),
        citations: vec![],
        fetched_memories: vec![],
        message: Some(api::message::Message::ToolCall(api::message::ToolCall {
            tool_call_id: Uuid::new_v4().to_string(),
            tool: Some(api::message::tool_call::Tool::RunShellCommand(
                api::message::tool_call::RunShellCommand {
                    command: command.to_string(),
                    is_read_only: false,
                    uses_pager: false,
                    citations: vec![],
                    is_risky: true,
                    wait_until_complete_value: Some(
                        api::message::tool_call::run_shell_command::WaitUntilCompleteValue::WaitUntilComplete(
                            true,
                        ),
                    ),
                    risk_category: 0,
                },
            )),
        })),
    };
    make_actions(vec![api::client_action::Action::AddMessagesToTask(
        api::client_action::AddMessagesToTask {
            task_id: task_id.to_string(),
            messages: vec![message],
        },
    )])
}

fn sse_incoming_text(chunk: &str) -> Option<String> {
    let parsed: ChatCompletionResponse = serde_json::from_str(chunk).ok()?;
    let choice = parsed.choices.first()?;
    let delta = choice
        .delta
        .as_ref()
        .and_then(|d| d.content.clone())
        .filter(|text| !text.is_empty());
    let message = choice
        .message
        .as_ref()
        .and_then(|m| m.content.clone())
        .filter(|text| !text.is_empty());
    delta.or(message)
}

fn apply_tool_call_deltas(
    acc: &mut Vec<OpenAIToolCall>,
    parsed: &ChatCompletionResponse,
) {
    let Some(choice) = parsed.choices.first() else {
        return;
    };
    if let Some(message) = choice.message.as_ref()
        && let Some(calls) = message.tool_calls.as_ref()
    {
        for call in calls {
            if !call.id.is_empty()
                && let Some(existing) = acc.iter_mut().find(|c| c.id == call.id)
            {
                merge_function_call(&mut existing.function, &call.function);
            } else if let Some(existing) = acc.iter_mut().rev().find(|c| c.id.is_empty()) {
                if !call.id.is_empty() {
                    existing.id = call.id.clone();
                }
                merge_function_call(&mut existing.function, &call.function);
            } else {
                acc.push(call.clone());
            }
        }
    }
    let Some(delta) = choice.delta.as_ref() else {
        return;
    };
    let Some(deltas) = delta.tool_calls.as_ref() else {
        return;
    };
    for d in deltas {
        while acc.len() <= d.index {
            acc.push(OpenAIToolCall {
                id: String::new(),
                type_: "function".into(),
                function: OpenAIFunctionCall::default(),
            });
        }
        let slot = &mut acc[d.index];
        if let Some(id) = d.id.as_ref()
            && !id.is_empty()
        {
            slot.id = id.clone();
        }
        if let Some(func) = d.function.as_ref() {
            merge_function_call(&mut slot.function, func);
        }
    }
}

fn merge_function_call(dst: &mut OpenAIFunctionCall, src: &OpenAIFunctionCall) {
    if dst.name.is_empty() && !src.name.is_empty() {
        dst.name = src.name.clone();
    }
    if !src.arguments.is_empty() {
        dst.arguments.push_str(&src.arguments);
    }
}

/// 调用中转站 chat/completions，返回 Warp ResponseEvent 流（全部 owned，无借用生命周期）。
pub async fn stream_chat_completion(
    user_message: String,
    model: String,
    api_key: String,
    conversation_id: String,
    request: &api::Request,
) -> Result<
    impl Stream<Item = Result<api::ResponseEvent, Arc<AIApiError>>> + Send + use<>,
    Arc<AIApiError>,
> {
    let request_id = Uuid::new_v4().to_string();
    let conversation_id = if conversation_id.is_empty() {
        Uuid::new_v4().to_string()
    } else {
        conversation_id
    };
    let existing_task_id = request
        .task_context
        .as_ref()
        .and_then(|context| context.tasks.first())
        .map(|task| task.id.clone());
    let task_id = existing_task_id
        .clone()
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let message_id = Uuid::new_v4().to_string();

    let context = format_terminal_context(request);
    let rules = format_project_rules(request);
    let pending_tool_results = collect_input_mcp_tool_results(request);
    let native_summaries = collect_input_native_tool_summaries(request);
    let (openai_tools, mcp_servers) = mcp_tools_from_request(request);
    let history = extract_history_messages(request);

    let mut system = SYSTEM_PROMPT.to_string();
    if !rules.is_empty() {
        system.push('\n');
        system.push_str(&rules);
    }
    if !openai_tools.is_empty() {
        system.push_str("\n可用 MCP 工具已通过 function calling 提供，请在需要时调用。\n");
    }

    let next_user = if !user_message.is_empty() {
        Some(if context.is_empty() {
            user_message
        } else {
            format!("{context}\n{user_message}")
        })
    } else if pending_tool_results.is_empty()
        && native_summaries.is_empty()
        && !context.is_empty()
    {
        Some(context)
    } else {
        None
    };
    let messages = assemble_openai_messages(
        system,
        history,
        next_user,
        pending_tool_results,
        native_summaries,
    );
    let known_shell_commands = collect_known_shell_commands(&messages);

    let request_body = ChatCompletionRequest {
        model,
        messages,
        stream: true,
        max_tokens: Some(8192),
        tools: if openai_tools.is_empty() {
            None
        } else {
            Some(openai_tools)
        },
        tool_choice: if mcp_servers.is_empty() {
            None
        } else {
            Some("auto".into())
        },
    };

    let url = format!("{}/chat/completions", base_url().trim_end_matches('/'));
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(180))
        .build()
        .map_err(|e| Arc::new(AIApiError::Other(anyhow::anyhow!("创建中转请求失败: {e}"))))?;
    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Content-Type", "application/json")
        .header("Accept", "text/event-stream")
        .json(&request_body)
        .send()
        .await
        .map_err(|_e| {
            Arc::new(AIApiError::Other(anyhow::anyhow!("中转站连不上，请检查网络。")))
        })?;

    if !response.status().is_success() {
        let status = response.status().as_u16();
        let body = response.text().await.unwrap_or_default();
        return Err(Arc::new(AIApiError::Other(anyhow::anyhow!(
            "{}",
            sanitize_relay_error(status, &body)
        ))));
    }

    let task_exists = existing_task_id.is_some();
    Ok(async_stream::stream! {
        yield Ok(make_init(&conversation_id, &request_id));
        yield Ok(make_agent_output_event(
            &task_id,
            &request_id,
            &message_id,
            "",
            task_exists,
        ));

        let mut buf = String::new();
        let mut accumulated = String::new();
        let mut tool_calls: Vec<OpenAIToolCall> = Vec::new();
        let mut halt_repetition = false;
        let mut byte_stream = response.bytes_stream();
        use futures::StreamExt as _;

        while let Some(chunk) = byte_stream.next().await {
            let Ok(bytes) = chunk else {
                yield Err(Arc::new(AIApiError::Other(anyhow::anyhow!(
                    "读取中转站回复时中断。"
                ))));
                return;
            };
            buf.push_str(&String::from_utf8_lossy(&bytes));
            while let Some(idx) = buf.find('\n') {
                let line = buf[..idx].trim_end_matches('\r').to_string();
                buf.drain(..=idx);
                let Some(data) = line.strip_prefix("data:") else {
                    continue;
                };
                let data = data.trim();
                if data.is_empty() || data == "[DONE]" {
                    continue;
                }
                if let Ok(parsed) = serde_json::from_str::<ChatCompletionResponse>(data) {
                    apply_tool_call_deltas(&mut tool_calls, &parsed);
                }
                if let Some(incoming) = sse_incoming_text(data)
                    && let Some(delta) = incremental_text_delta(&accumulated, &incoming)
                {
                    accumulated.push_str(&delta);
                    yield Ok(make_append_delta(&task_id, &request_id, &message_id, &delta));
                    if is_degenerate_tail(&accumulated) {
                        halt_repetition = true;
                        yield Ok(make_append_delta(
                            &task_id,
                            &request_id,
                            &message_id,
                            "\n\n（检测到重复输出，已停止。）",
                        ));
                        break;
                    }
                }
            }
            if halt_repetition {
                break;
            }
        }
        // Last SSE frame may omit the trailing newline.
        if !halt_repetition {
            let leftover = buf.trim().to_string();
            if let Some(data) = leftover.strip_prefix("data:") {
                let data = data.trim();
                if !data.is_empty() && data != "[DONE]" {
                    if let Ok(parsed) = serde_json::from_str::<ChatCompletionResponse>(data) {
                        apply_tool_call_deltas(&mut tool_calls, &parsed);
                    }
                    if let Some(incoming) = sse_incoming_text(data)
                        && let Some(delta) = incremental_text_delta(&accumulated, &incoming)
                    {
                        accumulated.push_str(&delta);
                        yield Ok(make_append_delta(
                            &task_id,
                            &request_id,
                            &message_id,
                            &delta,
                        ));
                        if is_degenerate_tail(&accumulated) {
                            halt_repetition = true;
                            yield Ok(make_append_delta(
                                &task_id,
                                &request_id,
                                &message_id,
                                "\n\n（检测到重复输出，已停止。）",
                            ));
                        }
                    }
                }
            }
        }

        let completed_tools: Vec<OpenAIToolCall> = tool_calls
            .into_iter()
            .filter(|c| !c.function.name.is_empty())
            .collect();

        let mut emitted_tool = false;
        let mut dropped_tools: Vec<String> = Vec::new();
        if !completed_tools.is_empty() {
            for call in completed_tools {
                let tool_call_id = if call.id.is_empty() {
                    Uuid::new_v4().to_string()
                } else {
                    call.id
                };
                let Some((server_id, tool_name)) =
                    parse_openai_tool_name(&call.function.name, &mcp_servers)
                else {
                    dropped_tools.push(call.function.name);
                    continue;
                };
                let args = serde_json::from_str::<serde_json::Value>(&call.function.arguments)
                    .ok()
                    .and_then(|v| json_to_prost_struct(&v));
                yield Ok(make_call_mcp_tool(
                    &task_id,
                    &request_id,
                    &tool_call_id,
                    server_id,
                    tool_name,
                    args,
                ));
                emitted_tool = true;
            }
        }
        if !dropped_tools.is_empty() && !emitted_tool {
            yield Ok(make_append_delta(
                &task_id,
                &request_id,
                &message_id,
                &format!(
                    "模型调用了未接入的工具（{}）。请换一种问法，或检查 MCP 配置。",
                    dropped_tools.join("、")
                ),
            ));
        } else if !emitted_tool && accumulated.trim().is_empty() {
            yield Ok(make_append_delta(
                &task_id,
                &request_id,
                &message_id,
                "（没有收到回复，请换个模型或检查中转站。）",
            ));
        } else if !emitted_tool && !halt_repetition && !is_degenerate_tail(&accumulated) {
            for command in extract_shell_commands(&accumulated) {
                if known_shell_commands.contains(&command) {
                    continue;
                }
                yield Ok(make_run_shell_command(&task_id, &request_id, &command));
            }
        }

        yield Ok(make_finished());
    })
}

/// 只认环境变量和 `~/.tzworp/token`，不读各厂商 BYOK 槽。
pub fn key_from_request_settings(
    _api_keys: Option<&api::request::settings::ApiKeys>,
    _custom: Option<&api::request::settings::CustomModelProviders>,
) -> Option<String> {
    resolve_api_key(None)
}

/// 解析模型 id：自定义 endpoint 的 config_key → slug。
pub fn resolve_model_name(
    model_id: &str,
    custom: Option<&api::request::settings::CustomModelProviders>,
) -> String {
    if let Some(ps) = custom {
        for p in &ps.providers {
            for m in &p.models {
                if m.config_key == model_id {
                    return m.slug.clone();
                }
            }
        }
    }
    if model_id == "auto" || model_id.is_empty() {
        return DEFAULT_MODEL.to_string();
    }
    model_id.to_string()
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
