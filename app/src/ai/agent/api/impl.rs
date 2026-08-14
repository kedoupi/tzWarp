use std::collections::HashMap;
use std::sync::Arc;

use futures_util::StreamExt;
use warp_core::features::FeatureFlag;
use warp_multi_agent_api as api;

use super::convert_to::convert_input;
use super::{ConvertToAPITypeError, RequestParams, ResponseStream};
use crate::ai::agent::redaction;
use crate::server::server_api::{AIApiError, ServerApi};
use crate::terminal::model::session::SessionType;

pub async fn generate_multi_agent_output(
    #[cfg_attr(feature = "team_relay", allow(unused_variables))] server_api: Arc<ServerApi>,
    mut params: RequestParams,
    cancellation_rx: futures::channel::oneshot::Receiver<()>,
) -> Result<ResponseStream, ConvertToAPITypeError> {
    #[cfg(feature = "team_relay")]
    let supported_tools: Vec<api::ToolType> = Vec::new();
    #[cfg(not(feature = "team_relay"))]
    let supported_tools = params
        .supported_tools_override
        .take()
        .unwrap_or_else(|| get_supported_tools(&params));
    #[cfg(feature = "team_relay")]
    let supported_cli_agent_tools: Vec<api::ToolType> = Vec::new();
    #[cfg(not(feature = "team_relay"))]
    let supported_cli_agent_tools = get_supported_cli_agent_tools(&params);
    let mut logging_metadata = HashMap::new();
    if let Some(metadata) = params.metadata {
        logging_metadata.insert(
            "is_autodetected_user_query".to_owned(),
            prost_types::Value {
                kind: Some(prost_types::value::Kind::BoolValue(
                    metadata.is_autodetected_user_query,
                )),
            },
        );
        logging_metadata.insert(
            "entrypoint".to_owned(),
            prost_types::Value {
                kind: Some(prost_types::value::Kind::StringValue(
                    metadata.entrypoint.entrypoint(),
                )),
            },
        );
        logging_metadata.insert(
            "is_auto_resume_after_error".to_owned(),
            prost_types::Value {
                kind: Some(prost_types::value::Kind::BoolValue(
                    metadata.is_auto_resume_after_error,
                )),
            },
        );
    }

    if params.should_redact_secrets {
        redaction::redact_inputs(&mut params.input);
    }

    let api_keys = api_keys_with_warp_credit_fallback_setting(
        params.api_keys,
        params.allow_use_of_warp_credits,
    );

    let request = api::Request {
        task_context: Some(api::request::TaskContext {
            tasks: params.tasks,
        }),
        input: Some(convert_input(params.input)?),
        settings: Some(api::request::Settings {
            model_config: Some(api::request::settings::ModelConfig {
                base: params.model.into(),
                cli_agent: params.cli_agent_model.into(),
                computer_use_agent: params.computer_use_model.into(),
                base_model_context_window_limit: params.context_window_limit.unwrap_or(0),
                ..Default::default()
            }),
            rules_enabled: params.is_memory_enabled,
            warp_drive_context_enabled: params.warp_drive_context_enabled
                && !cfg!(feature = "team_relay"),
            web_context_retrieval_enabled: !cfg!(feature = "team_relay"),
            supports_parallel_tool_calls: !cfg!(feature = "team_relay"),
            use_anthropic_text_editor_tools: false,
            planning_enabled: params.planning_enabled && !cfg!(feature = "team_relay"),
            supports_create_files: !cfg!(feature = "team_relay"),
            supported_tools: supported_tools.into_iter().map(Into::into).collect(),
            supports_long_running_commands: !cfg!(feature = "team_relay"),
            should_preserve_file_content_in_history: !cfg!(feature = "team_relay"),
            supports_todos_ui: !cfg!(feature = "team_relay"),
            supports_linked_code_blocks: !cfg!(feature = "team_relay")
                && FeatureFlag::LinkedCodeBlocks.is_enabled(),
            supports_started_child_task_message: !cfg!(feature = "team_relay"),
            supports_suggest_prompt: !cfg!(feature = "team_relay"),
            supports_read_image_files: !cfg!(feature = "team_relay")
                && FeatureFlag::ReadImageFiles.is_enabled(),
            supports_reasoning_message: !cfg!(feature = "team_relay"),
            api_keys,
            autonomy_level: params.autonomy_level.into(),
            isolation_level: params.isolation_level.into(),
            web_search_enabled: params.web_search_enabled && !cfg!(feature = "team_relay"),
            supported_cli_agent_tools: supported_cli_agent_tools
                .into_iter()
                .map(Into::into)
                .collect(),
            supports_v4a_file_diffs: !cfg!(feature = "team_relay")
                && FeatureFlag::V4AFileDiffs.is_enabled(),
            supports_summarization_via_message_replacement: !cfg!(feature = "team_relay")
                && FeatureFlag::SummarizationViaMessageReplacement.is_enabled(),
            supports_bundled_skills: !cfg!(feature = "team_relay")
                && FeatureFlag::BundledSkills.is_enabled(),
            supports_research_agent: params.research_agent_enabled && !cfg!(feature = "team_relay"),
            supports_orchestration_v2: !cfg!(feature = "team_relay")
                && supports_orchestration_v2(params.orchestration_enabled),
            supports_orchestration_runners: !cfg!(feature = "team_relay")
                && params.orchestration_enabled
                && FeatureFlag::CloudAgentRunners.is_enabled(),
            supports_background_computer_use: !cfg!(feature = "team_relay")
                && FeatureFlag::BackgroundComputerUse.is_enabled()
                && computer_use::background_supported(),
            custom_model_providers: params.custom_model_providers,
            custom_model_routers: params.custom_model_routers,
        }),
        metadata: Some(api::request::Metadata {
            logging: logging_metadata,
            conversation_id: params
                .conversation_token
                .as_ref()
                .map(|token| token.as_str().to_string())
                .unwrap_or_default(),
            ambient_agent_task_id: params
                .ambient_agent_task_id
                .map(|id| id.to_string())
                .unwrap_or_default(),
            forked_from_conversation_id: if params.conversation_token.is_none() {
                // We only include this param on our initial request to the server
                // (when the forked conversation has not been assigned a new id yet).
                params
                    .forked_from_conversation_token
                    .map(|token| token.as_str().to_string())
                    .unwrap_or_default()
            } else {
                String::new()
            },
            parent_agent_id: params.parent_agent_id.unwrap_or_default(),
            agent_name: params.agent_name.unwrap_or_default(),
        }),
        existing_suggestions: params
            .existing_suggestions
            .map(|suggestions| suggestions.into()),
        mcp_context: params.mcp_context.map(Into::into),
    };

    // tzWarp Team Relay：有中转站 API Key 时直连 OpenAI 兼容接口，不经 Warp Server。
    #[cfg(feature = "team_relay")]
    {
        use crate::ai::team_relay;

        let settings = request.settings.as_ref();
        let api_key = team_relay::key_from_request_settings(
            settings.and_then(|s| s.api_keys.as_ref()),
            settings.and_then(|s| s.custom_model_providers.as_ref()),
        );

        if let Some(api_key) = api_key {
            let user_message = team_relay::extract_user_message(&request).unwrap_or_default();
            if user_message.is_empty() && !team_relay::has_tool_results(&request) {
                let (tx, rx) = async_channel::unbounded();
                let _ = tx
                    .send(Err(Arc::new(AIApiError::Other(anyhow::anyhow!(
                        "未能读取用户消息，请输入内容后重试。"
                    )))))
                    .await;
                return Ok(Box::pin(rx));
            };

            let mid = settings
                .and_then(|s| s.model_config.as_ref())
                .map(|m| m.base.as_str())
                .unwrap_or("auto");
            let model = team_relay::resolve_model_name(
                mid,
                settings.and_then(|s| s.custom_model_providers.as_ref()),
            );

            let conversation_id = request
                .metadata
                .as_ref()
                .map(|m| m.conversation_id.clone())
                .unwrap_or_default();

            match team_relay::stream_chat_completion(
                user_message,
                model,
                api_key,
                conversation_id,
                &request,
            )
            .await
            {
                Ok(relay_stream) => {
                    let output_stream = relay_stream.take_until(cancellation_rx);
                    return Ok(Box::pin(output_stream));
                }
                Err(e) => {
                    log::error!("tzWarp team relay error: {e:#}");
                    let (tx, rx) = async_channel::unbounded();
                    let _ = tx.send(Err(e)).await;
                    return Ok(Box::pin(rx));
                }
            }
        } else {
            log::warn!("team_relay enabled but no API key; set TZAI_API_KEY / TEAM_RELAY_API_KEY");
            let (tx, rx) = async_channel::unbounded();
            let _ = tx
                .send(Err(Arc::new(AIApiError::Other(anyhow::anyhow!(
                    "未配置小桃子 API 密钥。请在「设置 → 智能体」中填写小桃子 API 密钥，或设置环境变量 TZAI_API_KEY。"
                )))))
                .await;
            return Ok(Box::pin(rx));
        }
    }

    #[cfg(not(feature = "team_relay"))]
    {
        let response_stream =
            warp_multi_agent_client::generate_multi_agent_output(server_api.as_ref(), &request)
                .await;
        match response_stream {
            Ok(stream) => {
                let output_stream = stream
                    .then(|result| async {
                        match result {
                            Ok(event) => Ok(event),
                            Err(error) => Err(convert_multi_agent_client_error(error).await),
                        }
                    })
                    .take_until(cancellation_rx);
                Ok(Box::pin(output_stream))
            }
            Err(e) => {
                let (tx, rx) = async_channel::unbounded();
                let _ = tx
                    .send(Err(convert_multi_agent_client_error(e).await))
                    .await;
                Ok(Box::pin(rx))
            }
        }
    }
}

#[cfg_attr(feature = "team_relay", allow(dead_code))]
async fn convert_multi_agent_client_error(
    error: warp_multi_agent_client::Error,
) -> Arc<AIApiError> {
    let error = match error {
        warp_multi_agent_client::Error::Authentication(error)
        | warp_multi_agent_client::Error::AmbientHeaders(error) => AIApiError::Other(error),
        warp_multi_agent_client::Error::Base64Decode(error) => {
            AIApiError::Other(anyhow::Error::from(error))
        }
        warp_multi_agent_client::Error::ProtobufDecode(error) => {
            AIApiError::Other(anyhow::Error::from(error))
        }
        warp_multi_agent_client::Error::EventSource(error) => {
            AIApiError::from_stream_error("GenerateMultiAgentOutput", *error).await
        }
    };
    Arc::new(error)
}

fn api_keys_with_warp_credit_fallback_setting(
    api_keys: Option<api::request::settings::ApiKeys>,
    allow_use_of_warp_credits: bool,
) -> Option<api::request::settings::ApiKeys> {
    match api_keys {
        Some(mut api_keys) => {
            api_keys.allow_use_of_warp_credits = allow_use_of_warp_credits;
            Some(api_keys)
        }
        None if allow_use_of_warp_credits => Some(api::request::settings::ApiKeys {
            allow_use_of_warp_credits: true,
            ..Default::default()
        }),
        None => None,
    }
}

fn supports_orchestration_v2(orchestration_enabled: bool) -> bool {
    orchestration_enabled
}

#[cfg_attr(feature = "team_relay", allow(dead_code))]
fn get_supported_tools(params: &RequestParams) -> Vec<api::ToolType> {
    let mut supported_tools = vec![
        api::ToolType::Grep,
        api::ToolType::FileGlob,
        api::ToolType::FileGlobV2,
        api::ToolType::ReadMcpResource,
        api::ToolType::CallMcpTool,
        api::ToolType::InitProject,
        api::ToolType::OpenCodeReview,
        api::ToolType::RunShellCommand,
        api::ToolType::SuggestNewConversation,
        api::ToolType::Subagent,
        api::ToolType::WriteToLongRunningShellCommand,
        api::ToolType::ReadShellCommandOutput,
        api::ToolType::ReadDocuments,
        api::ToolType::CreateDocuments,
        api::ToolType::EditDocuments,
        api::ToolType::SuggestPrompt,
    ];

    if FeatureFlag::ConversationsAsContext.is_enabled() {
        supported_tools.push(api::ToolType::FetchConversation);
    }

    match params.session_context.session_type() {
        None | Some(SessionType::Local) => {
            supported_tools.extend(&[
                api::ToolType::ReadFiles,
                api::ToolType::ApplyFileDiffs,
                api::ToolType::SearchCodebase,
            ]);

            if FeatureFlag::ArtifactCommand.is_enabled() {
                supported_tools.push(api::ToolType::UploadFileArtifact);
            }
        }
        Some(SessionType::WarpifiedRemote { host_id: Some(_) }) => {
            // Remote session with a known host — enable tools that route
            // through RemoteServerClient. The host_id is only populated
            // after a successful connection handshake, so its presence is a
            // sufficient proxy for client availability.
            supported_tools.extend(&[api::ToolType::ReadFiles, api::ToolType::ApplyFileDiffs]);
            if FeatureFlag::RemoteCodebaseIndexing.is_enabled() {
                supported_tools.push(api::ToolType::SearchCodebase);
            }
        }
        Some(SessionType::WarpifiedRemote { host_id: None }) => {
            // Feature flag off or not yet connected — no remote tools.
        }
    }

    if FeatureFlag::AgentModeComputerUse.is_enabled() && params.computer_use_enabled {
        supported_tools.extend(&[api::ToolType::UseComputer]);
        supported_tools.extend(&[api::ToolType::RequestComputerUse]);

        if FeatureFlag::VideoRecording.is_enabled() {
            supported_tools.extend(&[api::ToolType::StartRecording, api::ToolType::StopRecording]);
        }
    }

    supported_tools.push(api::ToolType::InsertReviewComments);

    if FeatureFlag::ListSkills.is_enabled() {
        supported_tools.push(api::ToolType::ReadSkill);
    }

    if params.orchestration_enabled {
        supported_tools.extend([api::ToolType::RunAgents, api::ToolType::SendMessageToAgent]);
        // Declare client-handled wait_for_events so the server doesn't
        // fall back to the legacy server-handled form.
        supported_tools.push(api::ToolType::WaitForEvents);
    }

    if FeatureFlag::AskUserQuestion.is_enabled() && params.ask_user_question_enabled {
        supported_tools.push(api::ToolType::AskUserQuestion);
    }

    supported_tools
}

#[cfg_attr(feature = "team_relay", allow(dead_code))]
fn get_supported_cli_agent_tools(params: &RequestParams) -> Vec<api::ToolType> {
    let mut supported_cli_agent_tools = vec![
        api::ToolType::WriteToLongRunningShellCommand,
        api::ToolType::ReadShellCommandOutput,
        api::ToolType::Grep,
        api::ToolType::FileGlob,
        api::ToolType::FileGlobV2,
    ];

    if FeatureFlag::TransferControlTool.is_enabled() {
        supported_cli_agent_tools.push(api::ToolType::TransferShellCommandControlToUser);
    }

    match params.session_context.session_type() {
        None | Some(SessionType::Local) => {
            supported_cli_agent_tools
                .extend(&[api::ToolType::ReadFiles, api::ToolType::SearchCodebase]);
        }
        Some(SessionType::WarpifiedRemote { host_id: Some(_) }) => {
            supported_cli_agent_tools.push(api::ToolType::ReadFiles);
            if FeatureFlag::RemoteCodebaseIndexing.is_enabled() {
                supported_cli_agent_tools.push(api::ToolType::SearchCodebase);
            }
        }
        Some(SessionType::WarpifiedRemote { host_id: None }) => {}
    }

    supported_cli_agent_tools
}

#[cfg(test)]
#[path = "impl_tests.rs"]
mod tests;
