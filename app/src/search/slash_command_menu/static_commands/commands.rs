use std::collections::HashMap;
use std::sync::LazyLock;

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use warp_core::features::FeatureFlag;

use super::{Availability, SlashCommandKind, SlashCommandSurfaces};
use crate::search::slash_command_menu::StaticCommand;
use crate::search::slash_command_menu::static_commands::Argument;
use crate::ui_components::color_dot;

pub static AGENT: LazyLock<StaticCommand> = LazyLock::new(|| StaticCommand {
    name: "/agent",
    description: "开始新对话",
    kind: SlashCommandKind::Agent,
    supported_surfaces: SlashCommandSurfaces::GuiAndTui {
        icon_path: "bundled/svg/warp-3.svg",
    },
    availability: Availability::AI_ENABLED.union(Availability::NOT_CLOUD_AGENT),
    auto_enter_ai_mode: false,
    argument: Some(Argument::optional().with_execute_on_selection()),
});

pub static CLOUD_AGENT: LazyLock<StaticCommand> = LazyLock::new(|| StaticCommand {
    name: "/cloud-agent",
    description: "Start a new cloud agent conversation",
    kind: SlashCommandKind::CloudAgent,
    supported_surfaces: SlashCommandSurfaces::GuiOnly {
        icon_path: "bundled/svg/warp-3.svg",
    },
    availability: Availability::AI_ENABLED.union(Availability::NOT_CLOUD_AGENT),
    auto_enter_ai_mode: false,
    argument: Some(Argument::optional().with_execute_on_selection()),
});

pub const ADD_MCP: StaticCommand = StaticCommand {
    name: "/add-mcp",
    description: "打开 MCP 设置并添加服务器",
    kind: SlashCommandKind::AddMcp,
    supported_surfaces: SlashCommandSurfaces::GuiOnly {
        icon_path: "bundled/svg/dataflow.svg",
    },
    availability: Availability::AI_ENABLED,
    auto_enter_ai_mode: false,
    argument: None,
};
pub const RESET_STATUSLINE: StaticCommand = StaticCommand {
    name: "/reset-statusline",
    description: "Reset the statusline to its default items and ordering",
    kind: SlashCommandKind::ResetStatusline,
    supported_surfaces: SlashCommandSurfaces::TuiOnly,
    availability: Availability::ALWAYS,
    auto_enter_ai_mode: false,
    argument: None,
};
pub const STATUSLINE: StaticCommand = StaticCommand {
    name: "/statusline",
    description: "Configure the statusline",
    kind: SlashCommandKind::Statusline,
    supported_surfaces: SlashCommandSurfaces::TuiOnly,
    availability: Availability::ALWAYS,
    auto_enter_ai_mode: false,
    argument: None,
};

pub const AUTO_APPROVE: StaticCommand = StaticCommand {
    name: "/auto-approve",
    description: "Toggle auto approve",
    kind: SlashCommandKind::AutoApprove,
    supported_surfaces: SlashCommandSurfaces::TuiOnly,
    availability: Availability::AGENT_VIEW
        .union(Availability::ACTIVE_CONVERSATION)
        .union(Availability::AI_ENABLED)
        .union(Availability::NOT_CLOUD_AGENT),
    auto_enter_ai_mode: false,
    argument: None,
};

pub const MCP: StaticCommand = StaticCommand {
    name: "/mcp",
    description: "View and manage MCP servers",
    kind: SlashCommandKind::Mcp,
    supported_surfaces: SlashCommandSurfaces::TuiOnly,
    availability: Availability::AI_ENABLED,
    auto_enter_ai_mode: false,
    argument: None,
};

pub const VIEW_LOGS: StaticCommand = StaticCommand {
    name: "/view-logs",
    description: "Bundle your logs into a zip archive",
    kind: SlashCommandKind::ViewLogs,
    supported_surfaces: SlashCommandSurfaces::TuiOnly,
    availability: Availability::ALWAYS,
    auto_enter_ai_mode: false,
    argument: None,
};

/// Starts the headless TUI voice-input session.
pub const VOICE: StaticCommand = StaticCommand {
    name: "/voice",
    description: "Start voice input (Ctrl-S)",
    kind: SlashCommandKind::Voice,
    supported_surfaces: SlashCommandSurfaces::TuiOnly,
    availability: Availability::AI_ENABLED.union(Availability::NOT_CLOUD_AGENT),
    auto_enter_ai_mode: false,
    argument: None,
};

pub const NATURAL_LANGUAGE_DETECTION: StaticCommand = StaticCommand {
    name: "/natural-language-detection",
    description: "Toggle natural language detection",
    kind: SlashCommandKind::NaturalLanguageDetection,
    supported_surfaces: SlashCommandSurfaces::TuiOnly,
    availability: Availability::AI_ENABLED,
    auto_enter_ai_mode: false,
    argument: None,
};

pub const API_KEYS: StaticCommand = StaticCommand {
    name: "/api-keys",
    description: "View and manage API keys",
    kind: SlashCommandKind::ApiKeys,
    supported_surfaces: SlashCommandSurfaces::TuiOnly,
    availability: Availability::AI_ENABLED,
    auto_enter_ai_mode: false,
    argument: None,
};

pub const CONNECT_GROK: StaticCommand = StaticCommand {
    name: "/connect-grok",
    description: "Connect your Grok (X Premium / SuperGrok) account",
    kind: SlashCommandKind::ConnectGrok,
    supported_surfaces: SlashCommandSurfaces::TuiOnly,
    availability: Availability::AI_ENABLED,
    auto_enter_ai_mode: false,
    argument: None,
};

pub const MANAGE_BILLING: StaticCommand = StaticCommand {
    name: "/manage-billing",
    description: "Open the team billing page in your browser",
    kind: SlashCommandKind::ManageBilling,
    supported_surfaces: SlashCommandSurfaces::TuiOnly,
    availability: Availability::ALWAYS,
    auto_enter_ai_mode: false,
    argument: None,
};
pub const UPGRADE: StaticCommand = StaticCommand {
    name: "/upgrade",
    description: "Open the Warp upgrade page in your browser",
    kind: SlashCommandKind::Upgrade,
    supported_surfaces: SlashCommandSurfaces::TuiOnly,
    availability: Availability::ALWAYS,
    auto_enter_ai_mode: false,
    argument: None,
};
pub const THEME: StaticCommand = StaticCommand {
    name: "/theme",
    description: "Set color theme",
    kind: SlashCommandKind::Theme,
    supported_surfaces: SlashCommandSurfaces::TuiOnly,
    availability: Availability::ALWAYS,
    auto_enter_ai_mode: false,
    argument: Some(Argument {
        hint_text: Some("<auto|light|dark>"),
        is_optional: false,
        should_execute_on_selection: false,
    }),
};

pub const EXIT: StaticCommand = StaticCommand {
    name: "/exit",
    description: "Exit Warp",
    kind: SlashCommandKind::Exit,
    supported_surfaces: SlashCommandSurfaces::TuiOnly,
    availability: Availability::ALWAYS,
    auto_enter_ai_mode: false,
    argument: None,
};

pub const STATUS: StaticCommand = StaticCommand {
    name: "/status",
    description: "Show session and account status",
    kind: SlashCommandKind::Status,
    supported_surfaces: SlashCommandSurfaces::TuiOnly,
    availability: Availability::ALWAYS,
    auto_enter_ai_mode: false,
    argument: None,
};

pub const LOGOUT: StaticCommand = StaticCommand {
    name: "/logout",
    description: "Log out of Warp",
    kind: SlashCommandKind::Logout,
    supported_surfaces: SlashCommandSurfaces::TuiOnly,
    availability: Availability::ALWAYS,
    auto_enter_ai_mode: false,
    argument: None,
};

pub static CREATE_ENVIRONMENT: LazyLock<StaticCommand> = LazyLock::new(|| StaticCommand {
    name: "/create-environment",
    description: "Create an Oz environment (Docker image + repos) via guided setup",
    kind: SlashCommandKind::CreateEnvironment,
    supported_surfaces: SlashCommandSurfaces::GuiOnly {
        icon_path: "bundled/svg/dataflow.svg",
    },
    availability: Availability::AI_ENABLED,
    auto_enter_ai_mode: false,
    argument: Some(
        Argument::optional()
            .with_hint_text("<optional repo paths or GitHub URLs>")
            .with_execute_on_selection(),
    ),
});

pub const CREATE_DOCKER_SANDBOX: StaticCommand = StaticCommand {
    name: "/docker-sandbox",
    description: "新建 Docker 沙箱终端会话",
    kind: SlashCommandKind::CreateDockerSandbox,
    supported_surfaces: SlashCommandSurfaces::GuiOnly {
        icon_path: "bundled/svg/docker.svg",
    },
    availability: Availability::LOCAL.union(Availability::AI_ENABLED),
    auto_enter_ai_mode: false,
    argument: None,
};

pub static CREATE_NEW_PROJECT: LazyLock<StaticCommand> = LazyLock::new(|| StaticCommand {
    name: "/create-new-project",
    description: "让智能体带你创建新的编程项目",
    kind: SlashCommandKind::CreateNewProject,
    supported_surfaces: SlashCommandSurfaces::GuiAndTui {
        icon_path: "bundled/svg/plus.svg",
    },
    availability: Availability::LOCAL | Availability::AI_ENABLED,
    auto_enter_ai_mode: true,
    argument: Some(Argument::required().with_hint_text("<describe what you want to build>")),
});

pub static EDIT_SKILL: LazyLock<StaticCommand> = LazyLock::new(|| StaticCommand {
    name: "/open-skill",
    description: "在编辑器中打开技能的 markdown 文件",
    kind: SlashCommandKind::EditSkill,
    supported_surfaces: SlashCommandSurfaces::GuiOnly {
        icon_path: "bundled/svg/file-code-02.svg",
    },
    availability: Availability::AI_ENABLED,
    auto_enter_ai_mode: false,
    argument: None,
});

pub static INVOKE_SKILL: LazyLock<StaticCommand> = LazyLock::new(|| StaticCommand {
    name: "/skills",
    description: "调用技能",
    kind: SlashCommandKind::InvokeSkill,
    supported_surfaces: SlashCommandSurfaces::GuiAndTui {
        icon_path: "bundled/svg/stars-01.svg",
    },
    availability: Availability::AI_ENABLED,
    auto_enter_ai_mode: false,
    argument: None,
});

pub static ADD_PROMPT: LazyLock<StaticCommand> = LazyLock::new(|| StaticCommand {
    name: "/add-prompt",
    description: "添加智能体提示词",
    kind: SlashCommandKind::AddPrompt,
    supported_surfaces: SlashCommandSurfaces::GuiOnly {
        icon_path: if FeatureFlag::AgentView.is_enabled() {
            "bundled/svg/prompt.svg"
        } else {
            "bundled/svg/agentmode.svg"
        },
    },
    availability: Availability::AI_ENABLED,
    auto_enter_ai_mode: false,
    argument: None,
});

pub const ADD_RULE: StaticCommand = StaticCommand {
    name: "/add-rule",
    description: "添加一条全局智能体规则",
    kind: SlashCommandKind::AddRule,
    supported_surfaces: SlashCommandSurfaces::GuiOnly {
        icon_path: "bundled/svg/book-open.svg",
    },
    availability: Availability::AI_ENABLED,
    auto_enter_ai_mode: false,
    argument: None,
};

pub static EDIT: LazyLock<StaticCommand> = LazyLock::new(|| StaticCommand {
    name: "/open-file",
    description: "在编辑器中打开文件",
    kind: SlashCommandKind::Edit,
    supported_surfaces: SlashCommandSurfaces::GuiOnly {
        icon_path: "bundled/svg/file-code-02.svg",
    },
    availability: Availability::LOCAL,
    auto_enter_ai_mode: false,
    argument: Some(
        Argument::optional().with_hint_text("<path/to/file[:line[:col]]> or \"@\" to search"),
    ),
});

pub static RENAME_TAB: LazyLock<StaticCommand> = LazyLock::new(|| StaticCommand {
    name: "/rename-tab",
    description: "重命名当前标签",
    kind: SlashCommandKind::RenameTab,
    supported_surfaces: SlashCommandSurfaces::GuiOnly {
        icon_path: "bundled/svg/pencil-line.svg",
    },
    availability: Availability::ALWAYS,
    auto_enter_ai_mode: false,
    argument: Some(Argument::required().with_hint_text("<tab name>")),
});

pub static RENAME_CONVERSATION: LazyLock<StaticCommand> = LazyLock::new(|| StaticCommand {
    name: "/rename-conversation",
    description: "重命名当前对话",
    kind: SlashCommandKind::RenameConversation,
    supported_surfaces: SlashCommandSurfaces::GuiOnly {
        icon_path: "bundled/svg/pencil-line.svg",
    },
    availability: Availability::AGENT_VIEW
        | Availability::ACTIVE_CONVERSATION
        | Availability::AI_ENABLED,
    auto_enter_ai_mode: false,
    argument: Some(Argument::required().with_hint_text("<new title>")),
});

static SET_TAB_COLOR_HINT: LazyLock<String> = LazyLock::new(|| {
    let mut hint = String::from("<");
    for color in color_dot::TAB_COLOR_OPTIONS {
        hint.push_str(&color.to_string().to_ascii_lowercase());
        hint.push('|');
    }
    hint.push_str("none>");
    hint
});

pub static SET_TAB_COLOR: LazyLock<StaticCommand> = LazyLock::new(|| StaticCommand {
    name: "/set-tab-color",
    description: "设置当前标签颜色",
    kind: SlashCommandKind::SetTabColor,
    supported_surfaces: SlashCommandSurfaces::GuiOnly {
        icon_path: "bundled/svg/ellipse.svg",
    },
    availability: Availability::ALWAYS,
    auto_enter_ai_mode: false,
    argument: Some(Argument::required().with_hint_text(SET_TAB_COLOR_HINT.as_str())),
});

pub static FORK: LazyLock<StaticCommand> = LazyLock::new(|| {
    let hint_text = "<optional prompt to send in forked conversation>";
    StaticCommand {
        name: "/fork",
        description: "从当前对话分支",
        kind: SlashCommandKind::Fork,
        supported_surfaces: SlashCommandSurfaces::GuiAndTui {
            icon_path: "bundled/svg/arrow-split.svg",
        },
        availability: Availability::AGENT_VIEW
            | Availability::ACTIVE_CONVERSATION
            | Availability::NO_LRC_CONTROL
            | Availability::AI_ENABLED,
        auto_enter_ai_mode: true,
        argument: Some(Argument::optional().with_hint_text(hint_text)),
    }
});

pub static MOVE_TO_CLOUD: LazyLock<StaticCommand> = LazyLock::new(|| StaticCommand {
    name: "/handoff",
    description: "Hand off this conversation to a cloud agent",
    kind: SlashCommandKind::MoveToCloud,
    supported_surfaces: SlashCommandSurfaces::GuiAndTui {
        icon_path: "bundled/svg/upload-cloud-01.svg",
    },
    availability: Availability::AGENT_VIEW
        | Availability::ACTIVE_CONVERSATION
        | Availability::AI_ENABLED
        | Availability::NOT_CLOUD_AGENT,
    auto_enter_ai_mode: false,
    argument: Some(
        Argument::optional()
            .with_hint_text("<optional follow-up prompt>")
            .with_execute_on_selection(),
    ),
});

pub const OPEN_CODE_REVIEW: StaticCommand = StaticCommand {
    name: "/open-code-review",
    description: "打开代码审查",
    kind: SlashCommandKind::OpenCodeReview,
    supported_surfaces: SlashCommandSurfaces::GuiOnly {
        icon_path: "bundled/svg/diff.svg",
    },
    availability: Availability::REPOSITORY,
    auto_enter_ai_mode: false,
    argument: None,
};

pub const INDEX: StaticCommand = StaticCommand {
    name: "/index",
    description: "索引此代码库",
    kind: SlashCommandKind::Index,
    supported_surfaces: SlashCommandSurfaces::GuiOnly {
        icon_path: "bundled/svg/find-all.svg",
    },
    availability: Availability::REPOSITORY
        .union(Availability::CODEBASE_CONTEXT)
        .union(Availability::AI_ENABLED),
    auto_enter_ai_mode: false,
    argument: None,
};

pub const INIT: StaticCommand = StaticCommand {
    name: "/init",
    description: "索引此代码库并生成 AGENTS.md",
    kind: SlashCommandKind::Init,
    supported_surfaces: SlashCommandSurfaces::GuiOnly {
        icon_path: "bundled/svg/warp-2.svg",
    },
    availability: Availability::REPOSITORY
        .union(Availability::AGENT_VIEW)
        .union(Availability::AI_ENABLED),
    auto_enter_ai_mode: true,
    argument: None,
};

pub const OPEN_PROJECT_RULES: StaticCommand = StaticCommand {
    name: "/open-project-rules",
    description: "打开项目规则文件（AGENTS.md）",
    kind: SlashCommandKind::OpenProjectRules,
    supported_surfaces: SlashCommandSurfaces::GuiOnly {
        icon_path: "bundled/svg/file-code-02.svg",
    },
    availability: Availability::REPOSITORY.union(Availability::AI_ENABLED),
    auto_enter_ai_mode: false,
    argument: None,
};

pub const OPEN_MCP_SERVERS: StaticCommand = StaticCommand {
    name: "/open-mcp-servers",
    description: "打开 MCP 服务器",
    kind: SlashCommandKind::OpenMcpServers,
    supported_surfaces: SlashCommandSurfaces::GuiOnly {
        icon_path: "bundled/svg/dataflow.svg",
    },
    availability: Availability::AI_ENABLED,
    auto_enter_ai_mode: false,
    argument: None,
};

pub const OPEN_SETTINGS_FILE: StaticCommand = StaticCommand {
    name: "/open-settings-file",
    description: "打开设置文件（TOML）",
    kind: SlashCommandKind::OpenSettingsFile,
    supported_surfaces: SlashCommandSurfaces::GuiOnly {
        icon_path: "bundled/svg/file-code-02.svg",
    },
    availability: Availability::LOCAL,
    auto_enter_ai_mode: false,
    argument: None,
};

pub const CHANGELOG: StaticCommand = StaticCommand {
    name: "/changelog",
    description: "打开最新更新日志",
    kind: SlashCommandKind::Changelog,
    supported_surfaces: SlashCommandSurfaces::GuiOnly {
        icon_path: "bundled/svg/book-open.svg",
    },
    availability: Availability::ALWAYS,
    auto_enter_ai_mode: false,
    argument: None,
};

// Accepts an optional argument so that buffers like `/feedback some text` still parse to
// this command (the trailing text is ignored on execution). Without this, typing any
// argument after `/feedback` would fall through and be treated as plain input.
pub static FEEDBACK: LazyLock<StaticCommand> = LazyLock::new(|| StaticCommand {
    name: "/feedback",
    description: "发送反馈",
    kind: SlashCommandKind::Feedback,
    supported_surfaces: SlashCommandSurfaces::GuiOnly {
        icon_path: "bundled/svg/feedback.svg",
    },
    availability: Availability::ALWAYS,
    auto_enter_ai_mode: false,
    argument: Some(Argument::optional().with_execute_on_selection()),
});

pub const OPEN_REPO: StaticCommand = StaticCommand {
    name: "/open-repo",
    description: "切换到其他已索引仓库",
    kind: SlashCommandKind::OpenRepo,
    supported_surfaces: SlashCommandSurfaces::GuiOnly {
        icon_path: "bundled/svg/folder.svg",
    },
    availability: Availability::LOCAL.union(Availability::AI_ENABLED),
    auto_enter_ai_mode: false,
    argument: None,
};

pub const OPEN_RULES: StaticCommand = StaticCommand {
    name: "/open-rules",
    description: "查看全部全局和项目规则",
    kind: SlashCommandKind::OpenRules,
    supported_surfaces: SlashCommandSurfaces::GuiOnly {
        icon_path: "bundled/svg/book-open.svg",
    },
    availability: Availability::AI_ENABLED,
    auto_enter_ai_mode: false,
    argument: None,
};

pub static NEW: LazyLock<StaticCommand> = LazyLock::new(|| StaticCommand {
    name: "/new",
    description: "开始新对话（等同 /agent）",
    kind: SlashCommandKind::New,
    supported_surfaces: SlashCommandSurfaces::GuiAndTui {
        icon_path: "bundled/svg/new-conversation.svg",
    },
    availability: Availability::NO_LRC_CONTROL
        | Availability::AI_ENABLED
        | Availability::NOT_CLOUD_AGENT,
    auto_enter_ai_mode: false,
    argument: Some(Argument::optional().with_execute_on_selection()),
});

pub const CLEAR: StaticCommand = StaticCommand {
    name: "/clear",
    description: "Clear the transcript and start a new conversation (alias for /agent)",
    kind: SlashCommandKind::Clear,
    supported_surfaces: SlashCommandSurfaces::TuiOnly,
    availability: Availability::NO_LRC_CONTROL
        .union(Availability::AI_ENABLED)
        .union(Availability::NOT_CLOUD_AGENT),
    auto_enter_ai_mode: false,
    argument: Some(Argument {
        hint_text: None,
        is_optional: true,
        should_execute_on_selection: true,
    }),
};

pub static MODEL: LazyLock<StaticCommand> = LazyLock::new(|| StaticCommand {
    name: "/model",
    description: "切换智能体所用模型",
    kind: SlashCommandKind::Model,
    supported_surfaces: SlashCommandSurfaces::GuiAndTui {
        icon_path: "bundled/svg/warp-3.svg",
    },
    availability: Availability::AGENT_VIEW | Availability::AI_ENABLED,
    auto_enter_ai_mode: true,
    argument: None,
});

pub static HOST: LazyLock<StaticCommand> = LazyLock::new(|| StaticCommand {
    name: "/host",
    description: "Switch the cloud agent execution host",
    kind: SlashCommandKind::Host,
    supported_surfaces: SlashCommandSurfaces::GuiOnly {
        icon_path: "bundled/svg/warp-3.svg",
    },
    availability: Availability::AGENT_VIEW
        | Availability::AI_ENABLED
        | Availability::CLOUD_MODE_V2_COMPOSER,
    auto_enter_ai_mode: true,
    argument: None,
});

pub static HARNESS: LazyLock<StaticCommand> = LazyLock::new(|| StaticCommand {
    name: "/harness",
    description: "Switch the cloud agent harness",
    kind: SlashCommandKind::Harness,
    supported_surfaces: SlashCommandSurfaces::GuiOnly {
        icon_path: "bundled/svg/warp-3.svg",
    },
    availability: Availability::AGENT_VIEW
        | Availability::AI_ENABLED
        | Availability::CLOUD_MODE_V2_COMPOSER,
    auto_enter_ai_mode: true,
    argument: None,
});

pub static ENVIRONMENT: LazyLock<StaticCommand> = LazyLock::new(|| StaticCommand {
    name: "/environment",
    description: "Switch the cloud agent environment",
    kind: SlashCommandKind::Environment,
    supported_surfaces: SlashCommandSurfaces::GuiOnly {
        icon_path: "bundled/svg/globe-04.svg",
    },
    availability: Availability::AGENT_VIEW
        | Availability::AI_ENABLED
        | Availability::CLOUD_MODE_V2_COMPOSER,
    auto_enter_ai_mode: true,
    argument: None,
});

pub static PROFILE: LazyLock<StaticCommand> = LazyLock::new(|| StaticCommand {
    name: "/profile",
    description: "切换当前执行配置文件",
    kind: SlashCommandKind::Profile,
    supported_surfaces: SlashCommandSurfaces::GuiOnly {
        icon_path: "bundled/svg/psychology.svg",
    },
    availability: Availability::AGENT_VIEW
        | Availability::AI_ENABLED
        | Availability::NOT_CLOUD_AGENT,
    auto_enter_ai_mode: true,
    argument: None,
});

pub const PLAN_NAME: &str = "/plan";

pub static PLAN: LazyLock<StaticCommand> = LazyLock::new(|| StaticCommand {
    name: PLAN_NAME,
    description: "让智能体先调研并制定任务计划",
    kind: SlashCommandKind::Plan,
    supported_surfaces: SlashCommandSurfaces::GuiAndTui {
        icon_path: "bundled/svg/file-06.svg",
    },
    availability: Availability::AI_ENABLED,
    auto_enter_ai_mode: true,
    argument: Some(Argument::optional().with_hint_text("<describe your task>")),
});

pub const ORCHESTRATE_NAME: &str = "/orchestrate";

pub static ORCHESTRATE: LazyLock<StaticCommand> = LazyLock::new(|| StaticCommand {
    name: ORCHESTRATE_NAME,
    description: "把任务拆成子任务，由多个智能体并行执行",
    kind: SlashCommandKind::Orchestrate,
    supported_surfaces: SlashCommandSurfaces::GuiAndTui {
        icon_path: "bundled/svg/warp-3.svg",
    },
    availability: Availability::LOCAL | Availability::AI_ENABLED,
    auto_enter_ai_mode: true,
    argument: Some(Argument::optional().with_hint_text("<describe your task>")),
});

/// If `query` starts with the given command `name` followed by a space,
/// returns the remainder of the query. Otherwise returns `None`.
pub fn strip_command_prefix(query: &str, name: &str) -> Option<String> {
    query
        .strip_prefix(name)
        .and_then(|rest| rest.strip_prefix(' '))
        .map(|rest| rest.to_string())
}

pub static COMPACT: LazyLock<StaticCommand> = LazyLock::new(|| StaticCommand {
    name: "/compact",
    description: "总结对话历史以腾出上下文空间",
    kind: SlashCommandKind::Compact,
    supported_surfaces: SlashCommandSurfaces::GuiAndTui {
        icon_path: "bundled/svg/collapse_content.svg",
    },
    availability: Availability::AGENT_VIEW
        | Availability::ACTIVE_CONVERSATION
        | Availability::NO_LRC_CONTROL
        | Availability::AI_ENABLED
        | Availability::NOT_CLOUD_AGENT,
    auto_enter_ai_mode: true,
    argument: Some(
        Argument::optional().with_hint_text("<optional custom summarization instructions>"),
    ),
});

pub static COMPACT_AND: LazyLock<StaticCommand> = LazyLock::new(|| StaticCommand {
    name: "/compact-and",
    description: "压缩对话后再发送后续提示",
    kind: SlashCommandKind::CompactAnd,
    supported_surfaces: SlashCommandSurfaces::GuiOnly {
        icon_path: "bundled/svg/collapse_content.svg",
    },
    availability: Availability::AGENT_VIEW
        | Availability::ACTIVE_CONVERSATION
        | Availability::NO_LRC_CONTROL
        | Availability::AI_ENABLED
        | Availability::NOT_CLOUD_AGENT,
    auto_enter_ai_mode: true,
    argument: Some(Argument::optional().with_hint_text("<prompt to send after compaction>")),
});

pub static QUEUE: LazyLock<StaticCommand> = LazyLock::new(|| StaticCommand {
    name: "/queue",
    description: "等智能体回复完成后再发送这条提示",
    kind: SlashCommandKind::Queue,
    supported_surfaces: SlashCommandSurfaces::GuiOnly {
        icon_path: "bundled/svg/clock-plus.svg",
    },
    availability: Availability::AGENT_VIEW
        | Availability::ACTIVE_CONVERSATION
        | Availability::AI_ENABLED
        | Availability::NOT_CLOUD_AGENT,
    auto_enter_ai_mode: true,
    argument: Some(Argument::required().with_hint_text("<prompt to send when agent is done>")),
});

pub static FORK_AND_COMPACT: LazyLock<StaticCommand> = LazyLock::new(|| {
    let hint_text = "<optional prompt to send after compaction>";
    StaticCommand {
        name: "/fork-and-compact",
        description: "分支当前对话并在副本中压缩",
        kind: SlashCommandKind::ForkAndCompact,
        supported_surfaces: SlashCommandSurfaces::GuiOnly {
            icon_path: "bundled/svg/fork_and_compact.svg",
        },
        availability: Availability::AGENT_VIEW
            | Availability::ACTIVE_CONVERSATION
            | Availability::NO_LRC_CONTROL
            | Availability::AI_ENABLED
            | Availability::NOT_CLOUD_AGENT,
        auto_enter_ai_mode: true,
        argument: Some(Argument::optional().with_hint_text(hint_text)),
    }
});

pub const FORK_FROM: StaticCommand = StaticCommand {
    name: "/fork-from",
    description: "从指定提问处分支出新对话",
    kind: SlashCommandKind::ForkFrom,
    supported_surfaces: SlashCommandSurfaces::GuiOnly {
        icon_path: "bundled/svg/arrow-split.svg",
    },
    availability: Availability::AGENT_VIEW
        .union(Availability::NO_LRC_CONTROL)
        .union(Availability::AI_ENABLED)
        .union(Availability::NOT_CLOUD_AGENT),
    auto_enter_ai_mode: true,
    argument: None,
};

pub static CONTINUE_LOCALLY: LazyLock<StaticCommand> = LazyLock::new(|| {
    let hint_text = "<optional prompt to send in local conversation>";
    StaticCommand {
        name: "/continue-locally",
        description: "Continue this cloud conversation locally",
        kind: SlashCommandKind::ContinueLocally,
        supported_surfaces: SlashCommandSurfaces::GuiOnly {
            icon_path: "bundled/svg/arrow-split.svg",
        },
        availability: Availability::AGENT_VIEW
            | Availability::ACTIVE_CONVERSATION
            | Availability::AI_ENABLED
            | Availability::CLOUD_AGENT,
        auto_enter_ai_mode: true,
        argument: Some(Argument::optional().with_hint_text(hint_text)),
    }
});

pub const USAGE: StaticCommand = StaticCommand {
    name: "/usage",
    description: "Open billing and usage settings",
    kind: SlashCommandKind::Usage,
    supported_surfaces: SlashCommandSurfaces::GuiOnly {
        icon_path: "bundled/svg/bar-chart-04.svg",
    },
    availability: Availability::AI_ENABLED,
    auto_enter_ai_mode: false,
    argument: None,
};

pub const REMOTE_CONTROL: StaticCommand = StaticCommand {
    name: "/remote-control",
    description: "Start remote control for this session",
    kind: SlashCommandKind::RemoteControl,
    supported_surfaces: SlashCommandSurfaces::GuiOnly {
        icon_path: "bundled/svg/phone-01.svg",
    },
    availability: Availability::AI_ENABLED.union(Availability::NOT_CLOUD_AGENT),
    auto_enter_ai_mode: false,
    argument: None,
};

pub const COST: StaticCommand = StaticCommand {
    name: "/cost",
    description: "Toggle credit usage details",
    kind: SlashCommandKind::Cost,
    supported_surfaces: SlashCommandSurfaces::GuiAndTui {
        icon_path: "bundled/svg/bar-chart-04.svg",
    },
    availability: Availability::AGENT_VIEW
        .union(Availability::AI_ENABLED)
        .union(Availability::NOT_CLOUD_AGENT),
    auto_enter_ai_mode: false,
    argument: None,
};

pub const CONVERSATIONS: StaticCommand = StaticCommand {
    name: "/conversations",
    description: "打开对话历史",
    kind: SlashCommandKind::Conversations,
    supported_surfaces: SlashCommandSurfaces::GuiAndTui {
        icon_path: "bundled/svg/conversation.svg",
    },
    availability: Availability::AI_ENABLED,
    auto_enter_ai_mode: false,
    argument: None,
};

pub static PROMPTS: LazyLock<StaticCommand> = LazyLock::new(|| StaticCommand {
    name: "/prompts",
    description: "搜索已保存的提示词",
    kind: SlashCommandKind::Prompts,
    supported_surfaces: SlashCommandSurfaces::GuiOnly {
        icon_path: "bundled/svg/prompt.svg",
    },
    availability: Availability::AI_ENABLED,
    auto_enter_ai_mode: false,
    argument: None,
});

pub const REWIND: StaticCommand = StaticCommand {
    name: "/rewind",
    description: "回退到对话中的某个位置",
    kind: SlashCommandKind::Rewind,
    supported_surfaces: SlashCommandSurfaces::GuiOnly {
        icon_path: "bundled/svg/clock-rewind.svg",
    },
    availability: Availability::AGENT_VIEW
        .union(Availability::AI_ENABLED)
        .union(Availability::NOT_CLOUD_AGENT),
    auto_enter_ai_mode: true,
    argument: None,
};

pub const EXPORT_TO_CLIPBOARD: StaticCommand = StaticCommand {
    name: "/export-to-clipboard",
    description: "把当前对话以 Markdown 复制到剪贴板",
    kind: SlashCommandKind::ExportToClipboard,
    supported_surfaces: SlashCommandSurfaces::GuiAndTui {
        icon_path: "bundled/svg/copy.svg",
    },
    availability: Availability::AGENT_VIEW
        .union(Availability::AI_ENABLED)
        .union(Availability::NOT_CLOUD_AGENT),
    auto_enter_ai_mode: true,
    argument: None,
};

pub static EXPORT_TO_FILE: LazyLock<StaticCommand> = LazyLock::new(|| StaticCommand {
    name: "/export-to-file",
    description: "把当前对话导出为 Markdown 文件",
    kind: SlashCommandKind::ExportToFile,
    supported_surfaces: SlashCommandSurfaces::GuiAndTui {
        icon_path: "bundled/svg/download-01.svg",
    },
    availability: Availability::AGENT_VIEW
        | Availability::AI_ENABLED
        | Availability::NOT_CLOUD_AGENT,
    auto_enter_ai_mode: true,
    argument: Some(Argument::optional().with_hint_text("<optional filename>")),
});

pub const VIM_MODE: StaticCommand = StaticCommand {
    name: "/vim-mode",
    description: "Toggle Vim mode",
    kind: SlashCommandKind::VimMode,
    supported_surfaces: SlashCommandSurfaces::TuiOnly,
    availability: Availability::ALWAYS,
    auto_enter_ai_mode: false,
    argument: None,
};

pub const COPY_DEBUGGING_ID: StaticCommand = StaticCommand {
    name: "/copy-debugging-id",
    description: "复制此对话的调试信息",
    kind: SlashCommandKind::CopyDebuggingId,
    supported_surfaces: SlashCommandSurfaces::GuiAndTui {
        icon_path: "bundled/svg/copy.svg",
    },
    availability: Availability::ACTIVE_CONVERSATION,
    auto_enter_ai_mode: false,
    argument: None,
};

pub static COMMAND_REGISTRY: LazyLock<Registry> = LazyLock::new(Registry::new);

/// A unique identifier for a static slash command.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
pub struct SlashCommandId(Uuid);

impl SlashCommandId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for SlashCommandId {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Registry {
    commands: HashMap<SlashCommandId, StaticCommand>,
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}

impl Registry {
    pub fn new() -> Self {
        let mut commands = HashMap::new();
        for command in all_commands_for_all_surfaces() {
            debug_assert!(
                !command
                    .availability
                    .contains(Availability::TERMINAL_VIEW | Availability::AGENT_VIEW),
                "command `{}` sets both TERMINAL_VIEW and AGENT_VIEW, which is unsatisfiable",
                command.name,
            );
            commands.insert(SlashCommandId::new(), command);
        }
        Self { commands }
    }

    pub fn all_commands_by_id(&self) -> impl Iterator<Item = (SlashCommandId, &StaticCommand)> {
        self.commands.iter().map(|(id, cmd)| (*id, cmd))
    }

    pub fn all_commands(&self) -> impl Iterator<Item = &StaticCommand> {
        self.commands.values()
    }

    pub fn get_command(&self, id: &SlashCommandId) -> Option<&StaticCommand> {
        self.commands.get(id)
    }

    pub fn get_command_with_name(&self, name: &str) -> Option<&StaticCommand> {
        self.commands.values().find(|command| command.name == name)
    }

    #[cfg(test)]
    pub fn get_command_id_with_name(&self, name: &str) -> Option<&SlashCommandId> {
        self.commands
            .iter()
            .find(|(_, command)| command.name == name)
            .map(|(id, _)| id)
    }
}

#[cfg(test)]
fn all_commands(settings_mode: settings::SettingsMode) -> Vec<StaticCommand> {
    all_commands_for_all_surfaces()
        .into_iter()
        .filter(|command| command.supports_surface(settings_mode))
        .collect()
}

fn all_commands_for_all_surfaces() -> Vec<StaticCommand> {
    let mut commands = vec![
        ADD_MCP,
        ADD_PROMPT.clone(),
        ADD_RULE,
        AUTO_APPROVE,
        EXIT,
        INDEX,
        INIT,
        API_KEYS,
        MCP,
        OPEN_PROJECT_RULES,
        OPEN_MCP_SERVERS,
        OPEN_RULES,
        AGENT.clone(),
        CLEAR,
        NEW.clone(),
        PLAN.clone(),
        RENAME_CONVERSATION.clone(),
        RENAME_TAB.clone(),
        SET_TAB_COLOR.clone(),
        STATUSLINE,
        RESET_STATUSLINE,
        NATURAL_LANGUAGE_DETECTION,
        THEME,
        VIM_MODE,
        CONVERSATIONS,
        EXPORT_TO_CLIPBOARD,
        COPY_DEBUGGING_ID,
        MODEL.clone(),
        STATUS,
        VIEW_LOGS,
        VOICE,
    ];

    #[cfg(not(feature = "team_relay"))]
    {
        commands.extend([
            COST,
            FEEDBACK.clone(),
            CONNECT_GROK,
            UPGRADE,
            MANAGE_BILLING,
            LOGOUT,
            USAGE,
        ]);
    }

    if FeatureFlag::LocalDockerSandbox.is_enabled() {
        commands.push(CREATE_DOCKER_SANDBOX);
    }

    if FeatureFlag::CreatingSharedSessions.is_enabled()
        && FeatureFlag::HOARemoteControl.is_enabled()
    {
        commands.push(REMOTE_CONTROL);
    }

    #[cfg(not(feature = "team_relay"))]
    if FeatureFlag::Changelog.is_enabled() {
        commands.push(CHANGELOG);
    }

    if FeatureFlag::AgentView.is_enabled() {
        commands.push(PROMPTS.clone());
    }

    commands.push(OPEN_CODE_REVIEW);

    #[cfg(not(feature = "team_relay"))]
    if FeatureFlag::CreateEnvironmentSlashCommand.is_enabled() {
        commands.push(CREATE_ENVIRONMENT.clone());
    }

    if FeatureFlag::CreateProjectFlow.is_enabled() {
        commands.push(CREATE_NEW_PROJECT.clone());
    }

    if FeatureFlag::SummarizationConversationCommand.is_enabled() {
        commands.push(COMPACT.clone());
        commands.push(COMPACT_AND.clone());
    }

    if FeatureFlag::QueueSlashCommand.is_enabled() {
        commands.push(QUEUE.clone());
    }

    if !cfg!(target_family = "wasm") {
        commands.extend([FORK.clone(), FORK_AND_COMPACT.clone()]);
        #[cfg(not(feature = "team_relay"))]
        commands.push(CONTINUE_LOCALLY.clone());

        if FeatureFlag::ForkFromCommand.is_enabled() {
            commands.push(FORK_FROM);
        }
    }

    if !cfg!(target_family = "wasm") {
        commands.extend([EDIT.clone(), EXPORT_TO_FILE.clone()]);
    }

    if FeatureFlag::ListSkills.is_enabled() && !cfg!(target_family = "wasm") {
        commands.push(EDIT_SKILL.clone());
        commands.push(INVOKE_SKILL.clone());
    }

    #[cfg(not(feature = "team_relay"))]
    if FeatureFlag::CloudMode.is_enabled() && FeatureFlag::CloudModeFromLocalSession.is_enabled() {
        commands.push(CLOUD_AGENT.clone());
    }

    #[cfg(not(feature = "team_relay"))]
    if FeatureFlag::OzHandoff.is_enabled()
        && FeatureFlag::HandoffLocalCloud.is_enabled()
        && cfg!(all(feature = "local_fs", not(target_family = "wasm")))
    {
        commands.push(MOVE_TO_CLOUD.clone());
    }

    if FeatureFlag::InlineProfileSelector.is_enabled() {
        commands.push(PROFILE.clone());
    }

    if FeatureFlag::RevertToCheckpoints.is_enabled() && FeatureFlag::RewindSlashCommand.is_enabled()
    {
        commands.push(REWIND);
    }

    if FeatureFlag::InlineRepoMenu.is_enabled() && !cfg!(target_family = "wasm") {
        commands.push(OPEN_REPO);
    }

    commands.push(ORCHESTRATE.clone());

    if FeatureFlag::SettingsFile.is_enabled() && cfg!(feature = "local_fs") {
        commands.push(OPEN_SETTINGS_FILE);
    }

    #[cfg(not(feature = "team_relay"))]
    if FeatureFlag::CloudModeInputV2.is_enabled() {
        commands.push(HOST.clone());
        commands.push(HARNESS.clone());
        commands.push(ENVIRONMENT.clone());
    }

    commands
}

#[cfg(test)]
#[path = "commands_tests.rs"]
mod tests;
