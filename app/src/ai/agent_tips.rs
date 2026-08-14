use std::path::Path;
use std::sync::LazyLock;
use std::time::Duration;

use ai::index::full_source_code_embedding::manager::CodebaseIndexManager;
use markdown_parser::FormattedTextFragment;
use warpui::r#async::{SpawnedFutureHandle, Timer};
use warpui::keymap::Keystroke;
use warpui::{AppContext, Entity, ModelContext, SingletonEntity};

use crate::ai::persisted_workspace::PersistedWorkspace;
use crate::palette::PaletteMode;
use crate::server::telemetry::PaletteSource;
use crate::settings::AISettings;
use crate::terminal::input::SET_INPUT_MODE_AGENT_ACTION_NAME;
use crate::terminal::view::init::{
    CANCEL_COMMAND_KEYBINDING, SELECT_PREVIOUS_BLOCK_ACTION_NAME,
    TOGGLE_AUTOEXECUTE_MODE_KEYBINDING,
};
use crate::util::bindings::trigger_to_keystroke;
use crate::workspace::WorkspaceAction;
use crate::workspace::view::{
    TOGGLE_COMMAND_PALETTE_KEYBINDING_NAME, TOGGLE_RIGHT_PANEL_BINDING_NAME,
};
use crate::workspaces::user_workspaces::UserWorkspaces;

/// Trait for tip implementations that can be displayed to users.
/// Tips provide helpful information with optional links and keybindings.
pub trait AITip: Clone {
    /// Returns the keystroke for this tip, if applicable.
    fn keystroke(&self, app: &AppContext) -> Option<Keystroke>;

    /// Returns the documentation link for this tip, if available.
    fn link(&self) -> Option<String>;

    /// Returns the raw description text for this tip.
    fn description(&self) -> &str;

    /// Converts the tip to formatted text fragments for rendering.
    /// Default implementation adds "Tip: " prefix and parses backtick-wrapped text as inline code.
    fn to_formatted_text(&self, _app: &AppContext) -> Vec<FormattedTextFragment> {
        let text = format!("提示：{}", self.description());

        // Style backtick-wrapped text as inline code
        let parts: Vec<&str> = text.split('`').collect();
        let mut fragments = Vec::new();
        for (i, part) in parts.iter().enumerate() {
            if part.is_empty() {
                continue;
            }
            if i % 2 == 0 {
                fragments.push(FormattedTextFragment::plain_text(part.to_string()));
            } else {
                fragments.push(FormattedTextFragment::inline_code(part.to_string()));
            }
        }
        fragments
    }

    /// Checks if this tip is applicable in the current context.
    /// Default implementation returns true (tip is always applicable).
    fn is_tip_applicable(
        &self,
        _current_working_directory: Option<&str>,
        _app: &AppContext,
    ) -> bool {
        true
    }
}

/// Kinds of agent tips for organizing and filtering.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AgentTipKind {
    CodebaseContext,
    WarpDrive,
    General,
    Mcp,
    SlashCommands,
    /// Tips about adding context (files, blocks, URLs, images, @-mentions, rules)
    Context,
    /// Tips about code editors, file trees, and code review panes
    Code,
    /// Tips about local-to-cloud handoff
    Handoff,
}

static DEFAULT_TIPS: LazyLock<Vec<AgentTip>> = LazyLock::new(|| {
    vec![
        AgentTip {
            description: "输入 `/` 打开斜杠命令菜单。".to_string(),
            link: Some("https://docs.warp.dev/agents/capabilities/slash-commands".to_string()),
            binding_name: None,
            action: None,
            kind: AgentTipKind::SlashCommands,
        },
        AgentTip {
            description: "按 <keybinding> 切换自然语言检测，在智能体和终端输入间切换。".to_string(),
            link: Some("https://docs.warp.dev/terminal/input/universal-input#input-modes".to_string()),
            binding_name: Some(SET_INPUT_MODE_AGENT_ACTION_NAME),
            action: None,
            kind: AgentTipKind::General,
        },
        AgentTip {
            description: "用 `/plan` <任务> 让智能体先做计划再执行。".to_string(),
            link: Some("https://docs.warp.dev/agents/capabilities/planning".to_string()),
            binding_name: None,
            action: None,
            kind: AgentTipKind::SlashCommands,
        },
        AgentTip {
            description: "按 <keybinding> 打开命令面板。".to_string(),
            link: Some("https://docs.warp.dev/terminal/command-palette".to_string()),
            binding_name: Some(TOGGLE_COMMAND_PALETTE_KEYBINDING_NAME),
            action: Some(WorkspaceAction::OpenPalette {
                mode: PaletteMode::Command,
                source: PaletteSource::AgentTip,
                query: None,
            }),
            kind: AgentTipKind::General,
        },
        AgentTip {
            description: "Store reusable workflows, notebooks, and prompts in your".to_string(),
            link: Some("https://docs.warp.dev/knowledge-and-collaboration/warp-drive".to_string()),
            binding_name: None,
            action: Some(WorkspaceAction::OpenWarpDrive),
            kind: AgentTipKind::WarpDrive,
        },
        AgentTip {
            description: "智能体运行时，输入新提示即可改方向。".to_string(),
            link: None,
            binding_name: None,
            action: None,
            kind: AgentTipKind::General,
        },
        AgentTip {
            description: "用 `@` 把文件、代码块等附加到提示里。".to_string(),
            link: Some("https://docs.warp.dev/agents/local-agents/agent-context/using-to-add-context".to_string()),
            binding_name: None,
            action: None,
            kind: AgentTipKind::Context,
        },
        AgentTip {
            description: "按 <keybinding> 把上一条命令输出附加为智能体上下文。".to_string(),
            link: Some("https://docs.warp.dev/agents/local-agents/agent-context/blocks-as-context#attaching-blocks-as-context".to_string()),
            binding_name: Some(SELECT_PREVIOUS_BLOCK_ACTION_NAME),
            action: None,
            kind: AgentTipKind::Context,
        },
        AgentTip {
            description: "用 `/init` 索引仓库，让智能体更好地理解代码库。".to_string(),
            link: Some("https://docs.warp.dev/agents/capabilities/codebase-context".to_string()),
            binding_name: None,
            action: None,
            kind: AgentTipKind::CodebaseContext,
        },
        AgentTip {
            description: "添加配置文件，按会话自定义权限和模型。".to_string(),
            link: Some("https://docs.warp.dev/agents/capabilities/agent-profiles-permissions".to_string()),
            binding_name: None,
            action: None,
            kind: AgentTipKind::General,
        },
        AgentTip {
            description: "右键代码块可从该处分支对话。".to_string(),
            link: Some("https://docs.warp.dev/agents/local-agents/interacting-with-agents/conversation-forking".to_string()),
            binding_name: None,
            action: None,
            kind: AgentTipKind::General,
        },
        AgentTip {
            description: "右键代码块可复制对话输出。".to_string(),
            link: Some("https://docs.warp.dev/terminal/blocks/block-actions#copy-input-output-of-block".to_string()),
            binding_name: None,
            action: None,
            kind: AgentTipKind::General,
        },
        AgentTip {
            description: "把图片拖进窗格即可附加为智能体上下文。".to_string(),
            link: Some("https://docs.warp.dev/agents/local-agents/agent-context/images-as-context".to_string()),
            binding_name: None,
            action: None,
            kind: AgentTipKind::Context,
        },
        AgentTip {
            description: "可以让智能体操作 node、python、postgres、gdb、vim 等交互式工具。".to_string(),
            link: Some("https://docs.warp.dev/agents/capabilities/full-terminal-use".to_string()),
            binding_name: None,
            action: None,
            kind: AgentTipKind::General,
        },
        AgentTip {
            description: "按 <keybinding> 打开代码审查面板，查看智能体的改动。".to_string(),
            link: Some("https://docs.warp.dev/code/code-review".to_string()),
            binding_name: Some(TOGGLE_RIGHT_PANEL_BINDING_NAME),
            action: None,
            kind: AgentTipKind::Code,
        },
        AgentTip {
            description: "用 `/add-mcp` 添加 MCP 服务器。".to_string(),
            link: Some("https://docs.warp.dev/agents/capabilities/mcp".to_string()),
            binding_name: None,
            action: None,
            kind: AgentTipKind::Mcp,
        },
        AgentTip {
            description: "用 `/open-mcp-servers` 查看 MCP 服务器。".to_string(),
            link: None,
            binding_name: None,
            action: None,
            kind: AgentTipKind::Mcp,
        },
        AgentTip {
            description: "`/create-environment` to turn a repo into a remote docker environment an agent can run in.".to_string(),
            link: Some("https://docs.warp.dev/reference/cli/integration-setup".to_string()),
            binding_name: None,
            action: None,
            kind: AgentTipKind::General,
        },
        AgentTip {
            description: "用 `/add-prompt` 创建可复用提示词。".to_string(),
            link: None,
            binding_name: None,
            action: None,
            kind: AgentTipKind::WarpDrive,
        },
        AgentTip {
            description: "用 `/add-rule` 创建全局智能体规则。".to_string(),
            link: Some("https://docs.warp.dev/agents/capabilities/rules".to_string()),
            binding_name: None,
            action: None,
            kind: AgentTipKind::Context,
        },
        AgentTip {
            description: "用 `/fork` 复制当前对话，也可带上新提示。".to_string(),
            link: Some("https://docs.warp.dev/agents/local-agents/interacting-with-agents/conversation-forking".to_string()),
            binding_name: None,
            action: None,
            kind: AgentTipKind::SlashCommands,
        },
        AgentTip {
            description: "用 `/open-code-review` 打开代码审查并查看智能体生成的差异。".to_string(),
            link: None,
            binding_name: None,
            action: Some(WorkspaceAction::ToggleRightPanel),
            kind: AgentTipKind::Code,
        },
        AgentTip {
            description: "用 `/new` 开始新的智能体对话。".to_string(),
            link: Some("https://docs.warp.dev/agents/local-agents/interacting-with-agents".to_string()),
            binding_name: None,
            action: None,
            kind: AgentTipKind::SlashCommands,
        },
        AgentTip {
            description: "用 `/compact` 总结当前对话，腾出上下文空间。".to_string(),
            link: None,
            binding_name: None,
            action: None,
            kind: AgentTipKind::SlashCommands,
        },
        AgentTip {
            description: "`/usage` to show your current AI credits usage.".to_string(),
            link: None,
            binding_name: None,
            action: None,
            kind: AgentTipKind::General,
        },
        AgentTip {
            description: "Use the `oz` command to run the Warp Agent in headless mode, useful for remote machines.".to_string(),
            link: Some("https://docs.warp.dev/reference/cli".to_string()),
            binding_name: None,
            action: None,
            kind: AgentTipKind::General,
        },
        AgentTip {
            description: "右键选中文本可附加为智能体上下文。".to_string(),
            link: Some("https://docs.warp.dev/agents/local-agents/agent-context/blocks-as-context#attaching-blocks-as-context".to_string()),
            binding_name: None,
            action: None,
            kind: AgentTipKind::Context,
        },
        AgentTip {
            description: "用 `AGENTS.md` 或 `CLAUDE.md` 作为项目规则。".to_string(),
            link: Some("https://docs.warp.dev/agents/capabilities/rules#project-rules-1".to_string()),
            binding_name: None,
            action: None,
            kind: AgentTipKind::Context,
        },
        AgentTip {
            description: "粘贴网址即可把该网页附加为智能体上下文。".to_string(),
            link: Some("https://docs.warp.dev/agents/local-agents/agent-context/urls-as-context".to_string()),
            binding_name: None,
            action: None,
            kind: AgentTipKind::Context,
        },
        AgentTip {
            description: "Warpify a remote SSH session to enable the Warp Agent inside that environment.".to_string(),
            link: Some("https://docs.warp.dev/terminal/warpify".to_string()),
            binding_name: None,
            action: None,
            kind: AgentTipKind::General,
        },
        AgentTip {
            description: "切换配置文件可快速更换模型和权限。".to_string(),
            link: Some("https://docs.warp.dev/agents/capabilities/agent-profiles-permissions".to_string()),
            binding_name: None,
            action: None,
            kind: AgentTipKind::General,
        },
        AgentTip {
            description: "用 `/init` 生成 `WARP.md` 并定义项目规则。".to_string(),
            link: Some("https://docs.warp.dev/agents/capabilities/rules".to_string()),
            binding_name: None,
            action: None,
            kind: AgentTipKind::SlashCommands,
        },
        AgentTip {
            description: "按 <keybinding> 在本会话中自动批准智能体的命令和改动。".to_string(),
            link: Some("https://docs.warp.dev/agents/capabilities/full-terminal-use#session-level-approvals".to_string()),
            binding_name: Some(TOGGLE_AUTOEXECUTE_MODE_KEYBINDING),
            action: None,
            kind: AgentTipKind::General,
        },
        AgentTip {
            description: "Type `&` or use the handoff chip to move a local conversation to the cloud.".to_string(),
            link: None,
            binding_name: None,
            action: None,
            kind: AgentTipKind::Handoff,
        },
        AgentTip {
            description: "Enable desktop notifications to get an alert when an agent needs your attention.".to_string(),
            link: Some("https://docs.warp.dev/platform/managing-cloud-agents#in-app-agent-notifications".to_string()),
            binding_name: None,
            action: None,
            kind: AgentTipKind::General,
        },
        AgentTip {
            description: "按 <keybinding> 取消当前智能体任务。".to_string(),
            link: None,
            binding_name: Some(CANCEL_COMMAND_KEYBINDING),
            action: None,
            kind: AgentTipKind::General,
        },
    ]
});

#[derive(Clone, Debug)]
pub struct AgentTip {
    /// The text that will be displayed to the user. This is parsed such that:
    /// "Tip: " is added as a prefix,
    /// "<keybinding>" is replaced with user-defined and platform-specific keybinding referenced by binding_name,
    /// `text` that is wrapped in backticks is formatted as inline code
    pub description: String,
    pub link: Option<String>,
    pub binding_name: Option<&'static str>,
    pub action: Option<WorkspaceAction>,
    /// The kind of the tip, used for filtering and organization
    pub kind: AgentTipKind,
}

impl AITip for AgentTip {
    fn keystroke(&self, app: &AppContext) -> Option<Keystroke> {
        let binding_name = self.binding_name?;

        // Special case: voice input uses settings, not editable bindings
        if binding_name == "FN" {
            return AISettings::as_ref(app).voice_input_toggle_key.keystroke();
        }

        if let Some(binding) = app.editable_bindings().find(|b| b.name == binding_name) {
            return trigger_to_keystroke(binding.trigger);
        }
        None
    }

    fn link(&self) -> Option<String> {
        self.link.clone()
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn to_formatted_text(&self, app: &AppContext) -> Vec<FormattedTextFragment> {
        let mut text = format!("Tip: {}", self.description);

        // Replace <keybinding> with the actual keybinding string
        if let Some(keystroke) = self.keystroke(app) {
            text = text.replace("<keybinding>", &keystroke.displayed());
        }

        // Style backtick-wrapped text as inline code
        let parts: Vec<&str> = text.split('`').collect();
        let mut fragments = Vec::new();
        for (i, part) in parts.iter().enumerate() {
            if part.is_empty() {
                continue;
            }
            if i % 2 == 0 {
                fragments.push(FormattedTextFragment::plain_text(part.to_string()));
            } else {
                fragments.push(FormattedTextFragment::inline_code(part.to_string()));
            }
        }

        fragments
    }

    fn is_tip_applicable(&self, current_working_directory: Option<&str>, app: &AppContext) -> bool {
        // Tips about indexing the repo are only applicable if the current directory is not already indexed.
        if matches!(self.kind, AgentTipKind::CodebaseContext) {
            let Some(cwd) = current_working_directory else {
                return true;
            };
            let Some(root) = PersistedWorkspace::as_ref(app).root_for_workspace(Path::new(cwd))
            else {
                return true;
            };
            return CodebaseIndexManager::as_ref(app)
                .get_codebase_index_status_for_path(root, app)
                .is_none();
        }
        // Handoff tips only apply when the feature is available and enabled.
        if matches!(self.kind, AgentTipKind::Handoff) {
            return AISettings::as_ref(app).is_cloud_handoff_enabled(app);
        }
        // Tips whose description references a keybinding placeholder should only be shown
        // when the keybinding is actually configured, so we never display the raw
        // "<keybinding>" string to users.
        if self.description.contains("<keybinding>") && self.keystroke(app).is_none() {
            return false;
        }
        true
    }
}

impl WorkspaceAction {
    pub fn display_text(&self) -> Option<String> {
        match self {
            WorkspaceAction::OpenPalette { .. } => Some("Open palette".to_string()),
            WorkspaceAction::OpenWarpDrive => Some("Warp Drive.".to_string()),
            WorkspaceAction::ToggleRightPanel => Some("Show diff view".to_string()),
            _ => None,
        }
    }
}

/// Helper function to build the list of agent tips, including the voice tip if enabled.
pub fn get_agent_tips(ctx: &AppContext) -> Vec<AgentTip> {
    let mut tips = DEFAULT_TIPS.clone();

    if cfg!(feature = "voice_input")
        && UserWorkspaces::as_ref(ctx).is_voice_enabled()
        && AISettings::as_ref(ctx).is_voice_input_enabled(ctx)
    {
        tips.push(AgentTip {
            description: "Hold <keybinding> to speak your prompt directly to the agent."
                .to_string(),
            link: Some(
                "https://docs.warp.dev/agents/local-agents/interacting-with-agents/voice"
                    .to_string(),
            ),
            binding_name: Some("FN"),
            action: None,
            kind: AgentTipKind::General,
        });
    }

    tips
}

/// A model for managing tips with cooldown logic.
/// Generic over any type implementing the AITip trait.
pub struct AITipModel<T: AITip> {
    tips: Vec<T>,
    current_tip: Option<T>,
    cooldown_handle: Option<SpawnedFutureHandle>,
}

impl<T: AITip + 'static> AITipModel<T> {
    /// Creates a new AITipModel with the given tips.
    /// Selects a random initial tip from the provided tips.
    ///
    /// # Panics
    /// Panics if the tips vector is empty.
    pub fn new(tips: Vec<T>) -> Self {
        use rand::seq::SliceRandom;
        debug_assert!(!tips.is_empty(), "AITipModel must have at least one tip");

        let mut rng = rand::thread_rng();
        let current_tip = tips.choose(&mut rng).cloned();

        Self {
            tips,
            current_tip,
            cooldown_handle: None,
        }
    }

    /// Returns the current tip, if one has been selected.
    pub fn current_tip(&self) -> Option<&T> {
        self.current_tip.as_ref()
    }
}

impl<T: AITip + 'static> Entity for AITipModel<T> {
    type Event = ();
}

// Specific implementation for AgentTip
impl AITipModel<AgentTip> {
    /// Creates a new AITipModel for AgentTips.
    /// This is the constructor used for the singleton model.
    pub fn new_for_agent_tips(ctx: &AppContext) -> Self {
        let tips = get_agent_tips(ctx);
        // Pick an applicable tip so we never show a raw "<keybinding>" placeholder on first render.
        let current_tip = Self::pick_random_applicable_tip(&tips, None, ctx);

        Self {
            tips,
            current_tip,
            cooldown_handle: None,
        }
    }

    /// Rebuilds the tip pool from current settings and invalidates the current tip
    /// if it is no longer applicable. Resets the cooldown timer so the revalidated
    /// tip is shown for the full cooldown period before the next rotation.
    pub fn revalidate_tips(&mut self, ctx: &mut ModelContext<Self>) {
        self.tips = get_agent_tips(ctx);

        // If the current tip is no longer in the pool or no longer applicable, pick a new one.
        let should_replace = self
            .current_tip
            .as_ref()
            .map(|current_tip| {
                let still_in_pool = self
                    .tips
                    .iter()
                    .any(|tip| tip.description == current_tip.description);

                !still_in_pool || !current_tip.is_tip_applicable(None, ctx)
            })
            .unwrap_or(true);

        if should_replace {
            let new_tip = Self::pick_random_applicable_tip(&self.tips, None, ctx);
            if new_tip.is_some() || self.current_tip.is_some() {
                self.current_tip = new_tip;
                self.reset_cooldown(ctx);
                ctx.notify();
            }
        }
    }

    /// Refreshes the current tip with a new random selection that is applicable
    /// for the given working directory.
    /// Only updates if not in cooldown period (60 seconds).
    pub fn maybe_refresh_tip(
        &mut self,
        current_working_directory: Option<&str>,
        ctx: &mut ModelContext<Self>,
    ) {
        // Don't update if cooldown is active
        if self.cooldown_handle.is_some() {
            return;
        }

        // Rebuild tips from current settings so changes are picked up.
        self.tips = get_agent_tips(ctx);

        self.current_tip =
            Self::pick_random_applicable_tip(&self.tips, current_working_directory, ctx);

        // Start 60-second cooldown
        let handle = ctx.spawn(
            async {
                Timer::after(Duration::from_secs(60)).await;
            },
            |me, _, _| {
                me.cooldown_handle = None;
            },
        );
        self.cooldown_handle = Some(handle);
        ctx.notify();
    }

    /// Picks a random applicable tip from the given pool, filtered by working directory.
    /// Returns `None` if no tips are applicable.
    fn pick_random_applicable_tip(
        tips: &[AgentTip],
        current_working_directory: Option<&str>,
        ctx: &AppContext,
    ) -> Option<AgentTip> {
        use rand::seq::SliceRandom;
        let available: Vec<&AgentTip> = tips
            .iter()
            .filter(|tip| tip.is_tip_applicable(current_working_directory, ctx))
            .collect();
        let mut rng = rand::thread_rng();
        available.choose(&mut rng).copied().cloned()
    }

    /// Resets the cooldown timer so the current tip is shown for the full
    /// cooldown period before the next rotation.
    fn reset_cooldown(&mut self, ctx: &mut ModelContext<Self>) {
        if let Some(handle) = self.cooldown_handle.take() {
            handle.abort();
        }
        let handle = ctx.spawn(
            async {
                Timer::after(Duration::from_secs(60)).await;
            },
            |me, _, _| {
                me.cooldown_handle = None;
            },
        );
        self.cooldown_handle = Some(handle);
    }
}

impl SingletonEntity for AITipModel<AgentTip> {}

// Specific implementation for CloudModeTip
impl AITipModel<crate::terminal::view::ambient_agent::CloudModeTip> {
    /// Refreshes the current tip with a new random selection.
    /// Only updates if not in cooldown period (60 seconds).
    pub fn maybe_refresh_tip(&mut self, ctx: &mut ModelContext<Self>) {
        // Don't update if cooldown is active
        if self.cooldown_handle.is_some() {
            return;
        }

        use rand::seq::SliceRandom;

        // Select a random tip
        let mut rng = rand::thread_rng();
        self.current_tip = self.tips.choose(&mut rng).cloned();

        // Start 60-second cooldown
        let handle = ctx.spawn(
            async {
                Timer::after(Duration::from_secs(60)).await;
            },
            |me, _, _| {
                me.cooldown_handle = None;
            },
        );
        self.cooldown_handle = Some(handle);
        ctx.notify();
    }

    /// Resets the cooldown timer without changing the current tip.
    /// This ensures the current tip will be shown for the full cooldown period.
    pub fn reset_cooldown(&mut self, ctx: &mut ModelContext<Self>) {
        // Cancel any existing cooldown
        if let Some(handle) = self.cooldown_handle.take() {
            handle.abort();
        }

        // Start a new 60-second cooldown
        let handle = ctx.spawn(
            async {
                Timer::after(Duration::from_secs(60)).await;
            },
            |me, _, _| {
                me.cooldown_handle = None;
            },
        );
        self.cooldown_handle = Some(handle);
    }
}
