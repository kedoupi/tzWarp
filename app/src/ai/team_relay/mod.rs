//! tzWarp Team Relay: 客户端直连团队 OpenAI 兼容中转站，不经 Warp Server。
//!
//! Base URL 默认锁定为 `https://tzai.kdp.cool/v1`，用户只需配置 API Key。

use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use futures::stream::Stream;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use warp_multi_agent_api as api;

use crate::server::server_api::AIApiError;

/// 团队中转站默认 Base URL（含 `/v1`）。
pub const DEFAULT_BASE_URL: &str = "https://tzai.kdp.cool/v1";

/// 设置/模型选择器中显示的名称。
pub const RELAY_DISPLAY_NAME: &str = "tzWarp 中转站";

/// 默认模型（中转站可用列表中的稳定项）。
pub const DEFAULT_MODEL: &str = "gpt-5.4-mini";

const SYSTEM_PROMPT: &str = "\
你是 tzWarp 终端里的智能体，直接帮用户处理当前工作目录里的事。

原则：
- 默认中文，简洁
- 结合用户提供的当前目录、选中内容和最近命令输出作答
- 需要执行的 shell 命令单独放在 markdown 代码块中，语言标记用 bash 或 sh，每块一条、不要 $ 前缀
- 命令会先展示给用户，确认后才运行
- 若提供了 MCP 工具，优先用工具查真实数据，不要编造
- 不确定就说不确定，不要编造文件内容或命令输出
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

/// 当前请求是否只带着工具结果（MCP 执行完后的续轮）。
pub fn has_tool_results(request: &api::Request) -> bool {
    !collect_input_tool_results(request).is_empty()
}

fn collect_input_tool_results(request: &api::Request) -> Vec<(String, String)> {
    use api::request::input::Type;
    use api::request::input::user_inputs::user_input::Input as UserInputType;

    let Some(input) = request.input.as_ref() else {
        return Vec::new();
    };
    let Some(Type::UserInputs(user_inputs)) = &input.r#type else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for ui in &user_inputs.inputs {
        if let Some(UserInputType::ToolCallResult(result)) = &ui.input {
            out.push((
                result.tool_call_id.clone(),
                format_tool_result_content(result),
            ));
        }
    }
    out
}

fn format_tool_result_content(result: &api::request::input::ToolCallResult) -> String {
    use api::request::input::tool_call_result::Result as ToolResult;
    match result.result.as_ref() {
        Some(ToolResult::CallMcpTool(mcp)) => format_mcp_tool_result(mcp),
        Some(other) => format!("{other:?}"),
        None => "（无结果）".into(),
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
                    out.push(ChatMessage::text("assistant", o.text.clone()));
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
                    out.push(ChatMessage {
                        role: "tool".into(),
                        content: Some(format_history_tool_result(result)),
                        tool_calls: None,
                        tool_call_id: Some(result.tool_call_id.clone()),
                    });
                }
                _ => {}
            }
        }
    }
    out
}

fn format_history_tool_result(result: &api::message::ToolCallResult) -> String {
    use api::message::tool_call_result::Result as R;
    match result.result.as_ref() {
        Some(R::CallMcpTool(mcp)) => format_mcp_tool_result(mcp),
        Some(other) => format!("{other:?}"),
        None => "（无结果）".into(),
    }
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

fn sse_delta_text(chunk: &str) -> Option<String> {
    let parsed: ChatCompletionResponse = serde_json::from_str(chunk).ok()?;
    parsed.choices.first().and_then(|c| {
        c.delta
            .as_ref()
            .and_then(|d| d.content.clone())
            .or_else(|| c.message.as_ref().and_then(|m| m.content.clone()))
    })
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
            if let Some(existing) = acc.iter_mut().find(|c| c.id == call.id && !call.id.is_empty())
            {
                if existing.function.name.is_empty() {
                    existing.function.name = call.function.name.clone();
                }
                if !call.function.arguments.is_empty() {
                    existing.function.arguments.push_str(&call.function.arguments);
                }
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
            if !func.name.is_empty() {
                slot.function.name = func.name.clone();
            }
            slot.function.arguments.push_str(&func.arguments);
        }
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
    let pending_tool_results = collect_input_tool_results(request);
    let (openai_tools, mcp_servers) = mcp_tools_from_request(request);
    let mut history = extract_history_messages(request);
    if !user_message.is_empty()
        && history.last().is_some_and(|m| {
            m.role == "user" && m.content.as_deref() == Some(user_message.as_str())
        })
    {
        history.pop();
    }

    let mut system = SYSTEM_PROMPT.to_string();
    if !rules.is_empty() {
        system.push('\n');
        system.push_str(&rules);
    }
    if !openai_tools.is_empty() {
        system.push_str("\n可用 MCP 工具已通过 function calling 提供，请在需要时调用。\n");
    }

    let mut messages = vec![ChatMessage::text("system", system)];
    messages.extend(history);
    if !user_message.is_empty() {
        messages.push(ChatMessage::text(
            "user",
            if context.is_empty() {
                user_message
            } else {
                format!("{context}\n{user_message}")
            },
        ));
    } else if !context.is_empty() && pending_tool_results.is_empty() {
        messages.push(ChatMessage::text("user", context));
    }
    for (tool_call_id, content) in pending_tool_results {
        if messages.iter().any(|m| m.tool_call_id.as_deref() == Some(tool_call_id.as_str())) {
            continue;
        }
        messages.push(ChatMessage {
            role: "tool".into(),
            content: Some(content),
            tool_calls: None,
            tool_call_id: Some(tool_call_id),
        });
    }

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
                if let Some(delta) = sse_delta_text(data)
                    && !delta.is_empty()
                {
                    accumulated.push_str(&delta);
                    yield Ok(make_append_delta(&task_id, &request_id, &message_id, &delta));
                }
            }
        }

        let completed_tools: Vec<OpenAIToolCall> = tool_calls
            .into_iter()
            .filter(|c| !c.function.name.is_empty())
            .collect();

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
            }
        } else if accumulated.trim().is_empty() {
            yield Ok(make_append_delta(
                &task_id,
                &request_id,
                &message_id,
                "（没有收到回复，请换个模型或检查中转站。）",
            ));
        } else {
            for command in extract_shell_commands(&accumulated) {
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
mod tests {
    use super::*;

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
}
