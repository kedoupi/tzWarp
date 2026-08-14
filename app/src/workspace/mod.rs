mod action;
mod active_session;
pub(crate) mod auto_handoff;
pub mod bonus_grant_notification_model;
#[cfg(target_os = "macos")]
pub(crate) mod cli_install;
mod close_session_confirmation_dialog;
pub(crate) mod cross_window_tab_drag;
pub mod delete_conversation_confirmation_dialog;
mod global_actions;
pub mod header_toolbar_editor;
pub mod header_toolbar_item;
pub mod hoa_onboarding;
mod home;
mod lightbox_view;
mod native_modal;
mod one_time_modal_model;
mod registry;
pub mod rewind_confirmation_dialog;
pub mod sync_inputs;
pub mod tab_group;
pub mod tab_settings;
mod toast_stack;
pub mod util;
pub mod view;

pub use action::{
    AutoCloudHandoffTrigger, CommandSearchOptions, InitContent, RestoreConversationLayout,
    TabContextMenuAnchor, VerticalTabsPaneContextMenuTarget, WorkspaceAction,
};
pub use active_session::ActiveSession;
pub use global_actions::{
    ForkAIConversationParams, ForkFromExchange, ForkedConversationDestination,
};
use serde::{Deserialize, Serialize};
pub use util::{PaneViewLocator, TabMovement, active_terminal_in_window};
pub use view::{
    NEW_SESSION_MENU_BUTTON_POSITION_ID, NEW_TAB_BUTTON_POSITION_ID, PANEL_HEADER_HEIGHT,
    TAB_BAR_HEIGHT, TOTAL_TAB_BAR_HEIGHT, WORKSPACE_PADDING, Workspace,
};
use warp_core::context_flag::ContextFlag;
use warpui::AppContext;
use warpui::accessibility::AccessibilityVerbosity;
use warpui::elements::DropTargetData;
use warpui::keymap::{BindingDescription, EditableBinding, FixedBinding};

use crate::ai::blocklist::NEW_AGENT_PANE_LABEL;
use crate::channel::{Channel, ChannelState};
use crate::features::FeatureFlag;
use crate::palette::PaletteMode;
use crate::server::telemetry::{AgentModeEntrypoint, PaletteSource};
use crate::settings_view::{self, SettingsSection, flags};
use crate::tab::{NewSessionMenuItem, uses_vertical_tabs};
use crate::util::bindings::{self, CustomAction, cmd_or_ctrl_shift, is_binding_pty_compliant};
use crate::{code, modal, notebooks, tab_configs};

// Helper function to access panel header corner radius from other modules
pub fn panel_header_corner_radius() -> warpui::elements::CornerRadius {
    warpui::elements::CornerRadius::with_top(warpui::elements::Radius::Pixels(8.))
}

pub use one_time_modal_model::OneTimeModalModel;
pub use registry::WorkspaceRegistry;
pub use toast_stack::{ToastStack, ToastStackEvent};

use crate::workspace::view::{
    LEFT_PANEL_AGENT_CONVERSATIONS_BINDING_NAME, LEFT_PANEL_GLOBAL_SEARCH_BINDING_NAME,
    LEFT_PANEL_PROJECT_EXPLORER_BINDING_NAME, LEFT_PANEL_WARP_DRIVE_BINDING_NAME,
    NEW_AGENT_TAB_BINDING_NAME, NEW_AMBIENT_AGENT_TAB_BINDING_NAME, NEW_FILE_BINDING_NAME,
    NEW_TAB_BINDING_NAME, NEW_TERMINAL_TAB_BINDING_NAME, OPEN_GLOBAL_SEARCH_BINDING_NAME,
    TOGGLE_CONVERSATION_LIST_VIEW_BINDING_NAME, TOGGLE_NOTIFICATION_MAILBOX_BINDING_NAME,
    TOGGLE_PROJECT_EXPLORER_BINDING_NAME, TOGGLE_RIGHT_PANEL_BINDING_NAME,
    TOGGLE_TAB_CONFIGS_MENU_BINDING_NAME, TOGGLE_VERTICAL_TABS_PANEL_BINDING_NAME,
    TOGGLE_WARP_DRIVE_BINDING_NAME,
};

pub fn init(app: &mut AppContext) {
    app.add_singleton_model(|_| WorkspaceRegistry::new());
    app.add_singleton_model(|_| cross_window_tab_drag::CrossWindowTabDrag::new());
    use warpui::keymap::macros::*;
    app.register_binding_validator::<Workspace>(is_binding_pty_compliant);

    modal::init(app);
    native_modal::init(app);
    lightbox_view::init(app);
    rewind_confirmation_dialog::init(app);
    delete_conversation_confirmation_dialog::init(app);
    crate::tab_configs::remove_confirmation_dialog::init(app);
    hoa_onboarding::init(app);
    tab_configs::session_config_modal::init(app);
    view::launch_modal::oz_launch::init(app);
    view::openwarp_launch_modal::init(app);
    view::orchestration_launch_modal::init(app);
    view::agent_cli_launch_modal::init(app);
    view::feature_intro_modal::init(app);
    view::auto_handoff_sleep_modal::init(app);
    view::cloud_agent_capacity_modal::init(app);
    view::codex_modal::init(app);
    view::free_ai_removal_modal::init(app);
    view::global_search::view::GlobalSearchView::init(app);
    view::right_panel::RightPanelView::init(app);
    header_toolbar_editor::init(app);
    view::conversation_list::view::register_conversation_list_view_bindings(app);

    settings_view::init_actions_from_parent_view(app, &id!("Workspace"), |settings_action| {
        WorkspaceAction::DispatchToSettingsTab(settings_action)
    });
    global_actions::init_global_actions(app);
    notebooks::init(app);
    code::init(app);
    sync_inputs::init(app);
    lsp::init(app);

    app.register_fixed_bindings([FixedBinding::empty(
        "导出调试信息",
        WorkspaceAction::DumpDebugInfo,
        id!("Workspace"),
    )]);
    app.register_fixed_bindings([
        FixedBinding::new(
            "escape",
            WorkspaceAction::DismissSessionConfigTabConfigChip,
            id!("Workspace") & id!(flags::SESSION_CONFIG_TAB_CONFIG_CHIP_OPEN),
        ),
        FixedBinding::new(
            "enter",
            WorkspaceAction::DismissSessionConfigTabConfigChip,
            id!("Workspace") & id!(flags::SESSION_CONFIG_TAB_CONFIG_CHIP_OPEN),
        ),
        // Feature intro never steals focus, so Escape must be handled at the workspace
        // level while the popover is open rather than on FeatureIntroModal itself.
        FixedBinding::new(
            "escape",
            WorkspaceAction::DismissFeatureIntroModal,
            id!("Workspace") & id!(flags::FEATURE_INTRO_MODAL_OPEN),
        ),
    ]);

    if ChannelState::enable_debug_features() {
        let crash_description = if cfg!(target_os = "macos") {
            "崩溃应用（Sentry-Cocoa 测试）"
        } else {
            "崩溃应用（Sentry-Native 测试）"
        };
        app.register_editable_bindings([
            EditableBinding::new("workspace:crash", crash_description, WorkspaceAction::Crash)
                .with_context_predicate(id!("Workspace")),
            EditableBinding::new(
                "workspace:log_review_comment_send_status_for_active_tab",
                "[调试] 记录当前标签页审查评论发送状态",
                WorkspaceAction::LogReviewCommentSendStatusForActiveTab,
            )
            .with_context_predicate(id!("Workspace")),
            EditableBinding::new(
                "workspace:panic",
                "触发 panic（Sentry 测试）",
                WorkspaceAction::Panic,
            )
            .with_context_predicate(id!("Workspace")),
            EditableBinding::new(
                "workspace:open_view_tree_debug_view",
                "打开视图树调试器",
                WorkspaceAction::OpenViewTreeDebugWindow,
            )
            .with_context_predicate(id!("Workspace")),
        ]);
        app.register_fixed_bindings([FixedBinding::empty(
            "[调试] 查看首次用户体验",
            WorkspaceAction::AddGetStartedTab,
            id!("Workspace"),
        )]);
        #[cfg(debug_assertions)]
        {
            // Debug actions for build plan migration modal (command palette only)
            app.register_editable_bindings([
                EditableBinding::new(
                    "workspace:open_build_plan_migration_modal",
                    "[调试] 打开构建计划迁移弹窗",
                    WorkspaceAction::OpenBuildPlanMigrationModal,
                )
                .with_context_predicate(id!("Workspace")),
                EditableBinding::new(
                    "workspace:reset_build_plan_migration_modal_state",
                    "[调试] 重置构建计划迁移弹窗状态",
                    WorkspaceAction::ResetBuildPlanMigrationModalState,
                )
                .with_context_predicate(id!("Workspace")),
                EditableBinding::new(
                    "workspace:debug_reset_aws_bedrock_login_banner_dismissed",
                    "[调试] 取消关闭 AWS 登录横幅",
                    WorkspaceAction::DebugResetAwsBedrockLoginBannerDismissed,
                )
                .with_context_predicate(id!("Workspace")),
                EditableBinding::new(
                    "workspace:open_oz_launch_modal",
                    "[调试] 打开 Oz 启动弹窗",
                    WorkspaceAction::OpenOzLaunchModal,
                )
                .with_context_predicate(id!("Workspace")),
                EditableBinding::new(
                    "workspace:reset_oz_launch_modal_state",
                    "[调试] 重置 Oz 启动弹窗状态",
                    WorkspaceAction::ResetOzLaunchModalState,
                )
                .with_context_predicate(id!("Workspace")),
                EditableBinding::new(
                    "workspace:open_openwarp_launch_modal",
                    "[调试] 打开 OpenWarp 启动弹窗",
                    WorkspaceAction::OpenOpenWarpLaunchModal,
                )
                .with_context_predicate(id!("Workspace")),
                EditableBinding::new(
                    "workspace:reset_openwarp_launch_modal_state",
                    "[调试] 重置 OpenWarp 启动弹窗状态",
                    WorkspaceAction::ResetOpenWarpLaunchModalState,
                )
                .with_context_predicate(id!("Workspace")),
                EditableBinding::new(
                    "workspace:open_orchestration_launch_modal",
                    "[调试] 打开编排启动弹窗",
                    WorkspaceAction::OpenOrchestrationLaunchModal,
                )
                .with_context_predicate(id!("Workspace")),
                EditableBinding::new(
                    "workspace:reset_orchestration_launch_modal_state",
                    "[调试] 重置编排启动弹窗状态",
                    WorkspaceAction::ResetOrchestrationLaunchModalState,
                )
                .with_context_predicate(id!("Workspace")),
                EditableBinding::new(
                    "workspace:open_agent_cli_launch_modal",
                    "[调试] 打开智能体 CLI 启动弹窗",
                    WorkspaceAction::OpenAgentCliLaunchModal,
                )
                .with_context_predicate(id!("Workspace")),
                EditableBinding::new(
                    "workspace:reset_agent_cli_launch_modal_state",
                    "[调试] 重置智能体 CLI 启动弹窗状态",
                    WorkspaceAction::ResetAgentCliLaunchModalState,
                )
                .with_context_predicate(id!("Workspace")),
                EditableBinding::new(
                    "workspace:open_feature_intro_modal",
                    "[调试] 打开功能介绍弹窗",
                    WorkspaceAction::OpenFeatureIntroModal,
                )
                .with_context_predicate(id!("Workspace")),
                EditableBinding::new(
                    "workspace:reset_feature_intro_modal_state",
                    "[调试] 重置功能介绍弹窗状态",
                    WorkspaceAction::ResetFeatureIntroModalState,
                )
                .with_context_predicate(id!("Workspace")),
                EditableBinding::new(
                    "workspace:open_auto_handoff_sleep_modal",
                    "[调试] 打开自动交接休眠弹窗",
                    WorkspaceAction::OpenAutoHandoffSleepModal,
                )
                .with_context_predicate(id!("Workspace")),
                EditableBinding::new(
                    "workspace:reset_auto_handoff_sleep_modal_state",
                    "[调试] 重置自动交接休眠弹窗状态",
                    WorkspaceAction::ResetAutoHandoffSleepModalState,
                )
                .with_context_predicate(id!("Workspace")),
                EditableBinding::new(
                    "workspace:trigger_auto_handoff_to_cloud",
                    "[调试] 触发自动交接至云端",
                    WorkspaceAction::TriggerAutoHandoffToCloud,
                )
                .with_context_predicate(id!("Workspace")),
                EditableBinding::new(
                    "workspace:open_free_ai_removal_modal",
                    "[调试] 打开免费 AI 移除弹窗",
                    WorkspaceAction::OpenFreeAiRemovalModal,
                )
                .with_context_predicate(id!("Workspace")),
                EditableBinding::new(
                    "workspace:reset_free_ai_removal_modal_state",
                    "[调试] 重置免费 AI 移除弹窗状态",
                    WorkspaceAction::ResetFreeAiRemovalModalState,
                )
                .with_context_predicate(id!("Workspace")),
                EditableBinding::new(
                    "workspace:install_opencode_warp_plugin",
                    "[调试] 安装 OpenCode 插件",
                    WorkspaceAction::InstallOpenCodeWarpPlugin,
                )
                .with_context_predicate(id!("Workspace")),
                EditableBinding::new(
                    "workspace:use_local_opencode_warp_plugin",
                    "[调试] 使用本地 OpenCode 插件（仅测试）",
                    WorkspaceAction::UseLocalOpenCodeWarpPlugin,
                )
                .with_context_predicate(id!("Workspace")),
                EditableBinding::new(
                    "workspace:open_session_config_modal",
                    "[调试] 打开会话配置弹窗",
                    WorkspaceAction::ShowSessionConfigModal,
                )
                .with_context_predicate(id!("Workspace")),
                EditableBinding::new(
                    "workspace:show_hoa_onboarding_flow",
                    "[调试] 启动 HOA 引导流程",
                    WorkspaceAction::ShowHoaOnboardingFlow,
                )
                .with_context_predicate(id!("Workspace")),
            ]);
        }
    }

    #[cfg(target_os = "macos")]
    app.register_editable_bindings([EditableBinding::new(
        "workspace:sample_process",
        "采样进程",
        WorkspaceAction::SampleProcess,
    )
    .with_context_predicate(id!("Workspace"))]);

    #[cfg(any(feature = "dhat_heap_profiling", feature = "heap_usage_tracking"))]
    {
        app.register_editable_bindings([EditableBinding::new(
            "workspace:dump_heap_profile",
            "将堆分析写入磁盘",
            WorkspaceAction::DumpHeapProfile,
        )
        .with_context_predicate(id!("Workspace"))]);
    }

    app.register_fixed_bindings([
        FixedBinding::custom(
            CustomAction::CycleNextSession,
            WorkspaceAction::CycleNextSession,
            "Switch to next tab",
            id!("Workspace") & id!("Workspace_MultipleTabs"),
        ),
        FixedBinding::custom(
            CustomAction::CyclePrevSession,
            WorkspaceAction::CyclePrevSession,
            "Switch to previous tab",
            id!("Workspace") & id!("Workspace_MultipleTabs"),
        ),
        FixedBinding::custom(
            CustomAction::AddWindow,
            WorkspaceAction::AddWindow,
            "Create New Window",
            id!("Workspace"),
        )
        .with_enabled(|| ContextFlag::CreateNewSession.is_enabled()),
    ]);

    app.register_editable_bindings([EditableBinding::new(
        NEW_FILE_BINDING_NAME,
        BindingDescription::new("新建文件"),
        WorkspaceAction::NewCodeFile,
    )
    .with_custom_action(CustomAction::NewFile)
    .with_context_predicate(id!("Workspace") & !id!("Workspace_ViewOnlySharedSession"))]);

    if FeatureFlag::UIZoom.is_enabled() {
        app.register_fixed_bindings([
            FixedBinding::custom(
                CustomAction::IncreaseZoom,
                WorkspaceAction::IncreaseZoom,
                "Zoom In",
                id!("Workspace"),
            )
            .with_group(bindings::BindingGroup::Settings.as_str()),
            FixedBinding::custom(
                CustomAction::DecreaseZoom,
                WorkspaceAction::DecreaseZoom,
                "Zoom Out",
                id!("Workspace"),
            )
            .with_group(bindings::BindingGroup::Settings.as_str()),
            FixedBinding::custom(
                CustomAction::ResetZoom,
                WorkspaceAction::ResetZoom,
                "Reset Zoom",
                id!("Workspace"),
            )
            .with_group(bindings::BindingGroup::Settings.as_str()),
        ]);
    } else {
        app.register_fixed_bindings([
            FixedBinding::custom(
                CustomAction::IncreaseFontSize,
                WorkspaceAction::IncreaseFontSize,
                "Increase font size",
                id!("Workspace"),
            )
            .with_group(bindings::BindingGroup::Settings.as_str()),
            FixedBinding::custom(
                CustomAction::DecreaseFontSize,
                WorkspaceAction::DecreaseFontSize,
                "Decrease font size",
                id!("Workspace"),
            )
            .with_group(bindings::BindingGroup::Settings.as_str()),
        ]);
    }

    if ContextFlag::LaunchConfigurations.is_enabled() {
        app.register_fixed_bindings([FixedBinding::custom(
            CustomAction::SaveCurrentConfig,
            WorkspaceAction::OpenLaunchConfigSaveModal,
            "Save new launch configuration",
            id!("Workspace"),
        )]);
    }

    if ChannelState::channel() == Channel::Integration {
        // Hack: Add explicit bindings for the tests, since the tests' injected
        // keypresses won't trigger Mac menu items. Unfortunately we can't use
        // cfg[test] because we are a separate process!
        app.register_fixed_bindings([
            FixedBinding::new(
                cmd_or_ctrl_shift("t"),
                WorkspaceAction::AddDefaultTab,
                id!("Workspace"),
            ),
            FixedBinding::new(
                cmd_or_ctrl_shift("p"),
                WorkspaceAction::TogglePalette {
                    mode: PaletteMode::Command,
                    source: PaletteSource::IntegrationTest,
                },
                id!("Workspace"),
            ),
            FixedBinding::new(
                "cmdorctrl-,",
                WorkspaceAction::ShowSettings,
                id!("Workspace"),
            ),
        ]);
    }

    if FeatureFlag::UIZoom.is_enabled() {
        app.register_editable_bindings([
            EditableBinding::new(
                "workspace:increase_zoom",
                "放大",
                WorkspaceAction::IncreaseZoom,
            )
            .with_context_predicate(id!("Workspace"))
            .with_group(bindings::BindingGroup::Settings.as_str())
            .with_key_binding("cmdorctrl-="),
            EditableBinding::new(
                "workspace:decrease_zoom",
                "缩小",
                WorkspaceAction::DecreaseZoom,
            )
            .with_context_predicate(id!("Workspace"))
            .with_group(bindings::BindingGroup::Settings.as_str())
            .with_key_binding("cmdorctrl--"),
            EditableBinding::new(
                "workspace:reset_zoom",
                "重置缩放",
                WorkspaceAction::ResetZoom,
            )
            .with_group(bindings::BindingGroup::Settings.as_str())
            .with_context_predicate(id!("Workspace")),
            EditableBinding::new(
                "workspace:increase_font_size",
                "增大字体",
                WorkspaceAction::IncreaseFontSize,
            )
            .with_context_predicate(id!("Workspace"))
            .with_group(bindings::BindingGroup::Settings.as_str())
            .with_key_binding("alt-shift->"),
            EditableBinding::new(
                "workspace:decrease_font_size",
                "减小字体",
                WorkspaceAction::DecreaseFontSize,
            )
            .with_context_predicate(id!("Workspace"))
            .with_group(bindings::BindingGroup::Settings.as_str())
            .with_key_binding("alt-shift-<"),
            EditableBinding::new(
                "workspace:reset_font_size",
                "重置字体大小",
                WorkspaceAction::ResetFontSize,
            )
            .with_group(bindings::BindingGroup::Settings.as_str())
            .with_context_predicate(id!("Workspace")),
        ]);
    } else {
        app.register_editable_bindings([
            EditableBinding::new(
                "workspace:increase_font_size",
                "增大字体",
                WorkspaceAction::IncreaseFontSize,
            )
            .with_context_predicate(id!("Workspace"))
            .with_group(bindings::BindingGroup::Settings.as_str())
            .with_key_binding("cmdorctrl-="),
            EditableBinding::new(
                "workspace:decrease_font_size",
                "减小字体",
                WorkspaceAction::DecreaseFontSize,
            )
            .with_context_predicate(id!("Workspace"))
            .with_group(bindings::BindingGroup::Settings.as_str())
            .with_key_binding("cmdorctrl--"),
            EditableBinding::new(
                "workspace:reset_font_size",
                "重置字体大小",
                WorkspaceAction::ResetFontSize,
            )
            .with_group(bindings::BindingGroup::Settings.as_str())
            .with_context_predicate(id!("Workspace"))
            .with_key_binding("cmdorctrl-0")
            .with_custom_action(CustomAction::ResetFontSize),
        ]);
    }

    app.register_fixed_bindings([
        // Menu dispatch for the "Open File Picker" custom action.
        FixedBinding::custom(
            CustomAction::ToggleProjectExplorer,
            WorkspaceAction::ToggleProjectExplorer,
            BindingDescription::new("切换项目浏览器")
                .with_custom_description(bindings::MAC_MENUS_CONTEXT, "项目浏览器"),
            id!("Workspace") & id!(flags::SHOW_PROJECT_EXPLORER),
        ),
    ]);

    app.register_editable_bindings([
        EditableBinding::new(
            "workspace:show_theme_chooser",
            "打开主题选择器",
            WorkspaceAction::ShowThemeChooserForActiveTheme,
        )
        .with_context_predicate(id!("Workspace"))
        .with_group(bindings::BindingGroup::Settings.as_str()),
        EditableBinding::new(
            TOGGLE_TAB_CONFIGS_MENU_BINDING_NAME,
            "打开标签页配置菜单",
            WorkspaceAction::ToggleTabConfigsMenu,
        )
        .with_context_predicate(id!("Workspace"))
        .with_mac_key_binding("cmd-ctrl-t")
        .with_linux_or_windows_key_binding("ctrl-alt-shift-T"),
        EditableBinding::new(
            "workspace:activate_first_tab",
            "切换到第 1 个标签页",
            WorkspaceAction::ActivateTabByNumber(1),
        )
        .with_context_predicate(id!("Workspace"))
        .with_group(bindings::BindingGroup::Navigation.as_str())
        .with_key_binding("cmdorctrl-1"),
        EditableBinding::new(
            "workspace:activate_second_tab",
            "切换到第 2 个标签页",
            WorkspaceAction::ActivateTabByNumber(2),
        )
        .with_context_predicate(id!("Workspace"))
        .with_group(bindings::BindingGroup::Navigation.as_str())
        .with_key_binding("cmdorctrl-2"),
        EditableBinding::new(
            "workspace:activate_third_tab",
            "切换到第 3 个标签页",
            WorkspaceAction::ActivateTabByNumber(3),
        )
        .with_context_predicate(id!("Workspace"))
        .with_group(bindings::BindingGroup::Navigation.as_str())
        .with_key_binding("cmdorctrl-3"),
        EditableBinding::new(
            "workspace:activate_fourth_tab",
            "切换到第 4 个标签页",
            WorkspaceAction::ActivateTabByNumber(4),
        )
        .with_context_predicate(id!("Workspace"))
        .with_group(bindings::BindingGroup::Navigation.as_str())
        .with_key_binding("cmdorctrl-4"),
        EditableBinding::new(
            "workspace:activate_fifth_tab",
            "切换到第 5 个标签页",
            WorkspaceAction::ActivateTabByNumber(5),
        )
        .with_context_predicate(id!("Workspace"))
        .with_group(bindings::BindingGroup::Navigation.as_str())
        .with_key_binding("cmdorctrl-5"),
        EditableBinding::new(
            "workspace:activate_sixth_tab",
            "切换到第 6 个标签页",
            WorkspaceAction::ActivateTabByNumber(6),
        )
        .with_context_predicate(id!("Workspace"))
        .with_group(bindings::BindingGroup::Navigation.as_str())
        .with_key_binding("cmdorctrl-6"),
        EditableBinding::new(
            "workspace:activate_seventh_tab",
            "切换到第 7 个标签页",
            WorkspaceAction::ActivateTabByNumber(7),
        )
        .with_context_predicate(id!("Workspace"))
        .with_group(bindings::BindingGroup::Navigation.as_str())
        .with_key_binding("cmdorctrl-7"),
        EditableBinding::new(
            "workspace:activate_eighth_tab",
            "切换到第 8 个标签页",
            WorkspaceAction::ActivateTabByNumber(8),
        )
        .with_context_predicate(id!("Workspace"))
        .with_group(bindings::BindingGroup::Navigation.as_str())
        .with_key_binding("cmdorctrl-8"),
        EditableBinding::new(
            "workspace:activate_last_tab",
            "切换到最后一个标签页",
            WorkspaceAction::ActivateLastTab,
        )
        .with_context_predicate(id!("Workspace"))
        .with_group(bindings::BindingGroup::Navigation.as_str())
        .with_key_binding("cmdorctrl-9"),
        EditableBinding::new(
            "workspace:activate_prev_tab",
            "激活上一个标签页",
            WorkspaceAction::ActivatePrevTab,
        )
        .with_context_predicate(
            id!("Workspace") & id!("Workspace_MultipleTabs") & !id!("Workspace_PaneDragging"),
        )
        .with_mac_key_binding("shift-cmd-{")
        .with_linux_or_windows_key_binding("ctrl-pageup"),
        EditableBinding::new(
            "workspace:activate_next_tab",
            "激活下一个标签页",
            WorkspaceAction::ActivateNextTab,
        )
        .with_context_predicate(
            id!("Workspace") & id!("Workspace_MultipleTabs") & !id!("Workspace_PaneDragging"),
        )
        .with_group(bindings::BindingGroup::Navigation.as_str())
        .with_mac_key_binding("shift-cmd-}")
        .with_linux_or_windows_key_binding("ctrl-pagedown"),
        EditableBinding::new(
            "pane_group:navigate_prev",
            "激活上一个窗格",
            WorkspaceAction::NavigatePrevPaneOrPanel,
        )
        .with_context_predicate(id!("Workspace"))
        .with_group(bindings::BindingGroup::Navigation.as_str())
        .with_custom_action(CustomAction::ActivatePreviousPane),
        EditableBinding::new(
            "pane_group:navigate_next",
            "激活下一个窗格",
            WorkspaceAction::NavigateNextPaneOrPanel,
        )
        .with_context_predicate(id!("Workspace"))
        .with_group(bindings::BindingGroup::Navigation.as_str())
        .with_custom_action(CustomAction::ActivateNextPane),
        #[cfg(not(feature = "team_relay"))]
        EditableBinding::new(
            "workspace:create_team_notebook",
            BindingDescription::new("新建团队笔记本")
                .with_custom_description(bindings::MAC_MENUS_CONTEXT, "新建团队笔记本"),
            WorkspaceAction::CreateTeamNotebook,
        )
        .with_custom_action(CustomAction::NewTeamNotebook)
        .with_context_predicate(
            id!("Workspace")
                & id!(flags::ENABLE_WARP_DRIVE)
                & id!("WarpDrive_BelongsToTeam")
                & id!("IsOnline"),
        )
        .with_group(bindings::BindingGroup::Notebooks.as_str()),
        #[cfg(not(feature = "team_relay"))]
        EditableBinding::new(
            "workspace:create_personal_notebook",
            BindingDescription::new("新建个人笔记本")
                .with_custom_description(bindings::MAC_MENUS_CONTEXT, "新建个人笔记本"),
            WorkspaceAction::CreatePersonalNotebook,
        )
        .with_group(bindings::BindingGroup::Notebooks.as_str())
        .with_custom_action(CustomAction::NewPersonalNotebook)
        .with_context_predicate(id!("Workspace") & id!(flags::ENABLE_WARP_DRIVE)),
        #[cfg(not(feature = "team_relay"))]
        EditableBinding::new(
            "workspace:create_team_workflow",
            BindingDescription::new("新建团队工作流")
                .with_custom_description(bindings::MAC_MENUS_CONTEXT, "新建团队工作流"),
            WorkspaceAction::CreateTeamWorkflow,
        )
        .with_custom_action(CustomAction::NewTeamWorkflow)
        .with_context_predicate(
            id!("Workspace")
                & id!(flags::ENABLE_WARP_DRIVE)
                & id!("IsOnline")
                & id!("WarpDrive_BelongsToTeam"),
        )
        .with_group(bindings::BindingGroup::Workflow.as_str()),
        #[cfg(not(feature = "team_relay"))]
        EditableBinding::new(
            "workspace:create_personal_workflow",
            BindingDescription::new("新建个人工作流")
                .with_custom_description(bindings::MAC_MENUS_CONTEXT, "新建个人工作流"),
            WorkspaceAction::CreatePersonalWorkflow,
        )
        .with_group(bindings::BindingGroup::Workflow.as_str())
        .with_custom_action(CustomAction::NewPersonalWorkflow)
        .with_context_predicate(id!("Workspace") & id!(flags::ENABLE_WARP_DRIVE)),
        #[cfg(not(feature = "team_relay"))]
        EditableBinding::new(
            "workspace:create_team_folder",
            BindingDescription::new("新建团队文件夹")
                .with_custom_description(bindings::MAC_MENUS_CONTEXT, "新建团队文件夹"),
            WorkspaceAction::CreateTeamFolder,
        )
        .with_context_predicate(
            id!("Workspace")
                & id!(flags::ENABLE_WARP_DRIVE)
                & id!("IsOnline")
                & id!("WarpDrive_BelongsToTeam"),
        )
        .with_group(bindings::BindingGroup::Folders.as_str()),
        #[cfg(not(feature = "team_relay"))]
        EditableBinding::new(
            "workspace:create_personal_folder",
            BindingDescription::new("新建个人文件夹")
                .with_custom_description(bindings::MAC_MENUS_CONTEXT, "新建个人文件夹"),
            WorkspaceAction::CreatePersonalFolder,
        )
        .with_group(bindings::BindingGroup::Folders.as_str())
        .with_context_predicate(id!("Workspace") & id!(flags::ENABLE_WARP_DRIVE) & id!("IsOnline")),
        EditableBinding::new(
            NEW_TAB_BINDING_NAME,
            BindingDescription::new("新建标签页"),
            WorkspaceAction::AddDefaultTab,
        )
        .with_context_predicate(id!("Workspace") & !id!("Workspace_PaneDragging"))
        .with_custom_action(CustomAction::NewTab)
        .with_enabled(|| ContextFlag::CreateNewSession.is_enabled()),
        EditableBinding::new(
            NEW_TERMINAL_TAB_BINDING_NAME,
            BindingDescription::new("新建终端标签页"),
            WorkspaceAction::AddTerminalTab {
                hide_homepage: false,
            },
        )
        .with_context_predicate(id!("Workspace") & !id!("Workspace_PaneDragging"))
        .with_custom_action(CustomAction::NewTerminalTab)
        .with_enabled(|| ContextFlag::CreateNewSession.is_enabled()),
        EditableBinding::new(
            NEW_AGENT_TAB_BINDING_NAME,
            BindingDescription::new("新建智能体标签页"),
            WorkspaceAction::AddAgentTab,
        )
        .with_group(bindings::BindingGroup::WarpAi.as_str())
        .with_custom_action(CustomAction::NewAgentTab)
        .with_context_predicate(
            id!("Workspace") & id!(flags::IS_ANY_AI_ENABLED) & !id!("Workspace_PaneDragging"),
        ),
        EditableBinding::new(
            NEW_AMBIENT_AGENT_TAB_BINDING_NAME,
            BindingDescription::new("新建云端智能体标签页"),
            WorkspaceAction::AddAmbientAgentTab,
        )
        .with_group(bindings::BindingGroup::WarpAi.as_str())
        .with_context_predicate(
            id!("Workspace") & id!(flags::IS_ANY_AI_ENABLED) & !id!("Workspace_PaneDragging"),
        )
        .with_enabled(|| {
            !cfg!(feature = "team_relay")
                && FeatureFlag::AgentView.is_enabled()
                && FeatureFlag::CloudMode.is_enabled()
        }),
        EditableBinding::new(
            "workspace:toggle_left_panel",
            BindingDescription::new("打开左侧面板"),
            WorkspaceAction::ToggleLeftPanel,
        )
        .with_context_predicate(id!("Workspace"))
        .with_custom_action(CustomAction::ToggleWarpDrive),
        EditableBinding::new(
            TOGGLE_RIGHT_PANEL_BINDING_NAME,
            BindingDescription::new("切换代码审查")
                .with_custom_description(bindings::MAC_MENUS_CONTEXT, "切换代码审查"),
            WorkspaceAction::ToggleRightPanel,
        )
        .with_enabled(|| cfg!(feature = "local_fs"))
        .with_context_predicate(id!("Workspace"))
        .with_mac_key_binding("cmd-shift-+")
        .with_linux_or_windows_key_binding("ctrl-shift-+"),
        EditableBinding::new(
            TOGGLE_VERTICAL_TABS_PANEL_BINDING_NAME,
            BindingDescription::new("切换垂直标签页面板")
                .with_custom_description(bindings::MAC_MENUS_CONTEXT, "切换垂直标签页面板"),
            WorkspaceAction::ToggleVerticalTabsPanel,
        )
        .with_context_predicate(id!("Workspace") & id!(flags::USE_VERTICAL_TABS_FLAG))
        .with_group(bindings::BindingGroup::Navigation.as_str())
        .with_enabled(|| FeatureFlag::VerticalTabs.is_enabled())
        .with_key_binding(cmd_or_ctrl_shift("b")),
        EditableBinding::new(
            LEFT_PANEL_PROJECT_EXPLORER_BINDING_NAME,
            BindingDescription::new("左侧面板：项目浏览器"),
            WorkspaceAction::ToggleProjectExplorer,
        )
        .with_group(bindings::BindingGroup::Navigation.as_str())
        .with_context_predicate(id!("Workspace") & id!(flags::SHOW_PROJECT_EXPLORER))
        .with_custom_action(CustomAction::ToggleProjectExplorer),
        EditableBinding::new(
            LEFT_PANEL_AGENT_CONVERSATIONS_BINDING_NAME,
            BindingDescription::new("左侧面板：智能体对话"),
            WorkspaceAction::ToggleConversationListView,
        )
        .with_group(bindings::BindingGroup::Navigation.as_str())
        .with_context_predicate(id!("Workspace") & id!(flags::SHOW_CONVERSATION_HISTORY))
        .with_enabled(|| FeatureFlag::AgentViewConversationListView.is_enabled())
        .with_custom_action(CustomAction::ToggleConversationListView),
        EditableBinding::new(
            LEFT_PANEL_GLOBAL_SEARCH_BINDING_NAME,
            BindingDescription::new("左侧面板：全局搜索"),
            WorkspaceAction::ToggleGlobalSearch,
        )
        .with_group(bindings::BindingGroup::Navigation.as_str())
        .with_context_predicate(id!("Workspace") & id!(flags::SHOW_GLOBAL_SEARCH))
        .with_enabled(|| FeatureFlag::GlobalSearch.is_enabled())
        .with_custom_action(CustomAction::ToggleGlobalSearch),
        EditableBinding::new(
            "file_tree:toggle_hidden_files",
            BindingDescription::new("切换项目浏览器中的隐藏文件"),
            WorkspaceAction::ToggleHiddenFiles,
        )
        .with_group(bindings::BindingGroup::Navigation.as_str())
        .with_context_predicate(id!("Workspace") & id!(flags::SHOW_PROJECT_EXPLORER))
        .with_mac_key_binding("cmd-shift->")
        .with_linux_or_windows_key_binding("ctrl-shift->"),
        EditableBinding::new(
            LEFT_PANEL_WARP_DRIVE_BINDING_NAME,
            BindingDescription::new("左侧面板：Warp Drive"),
            WorkspaceAction::ToggleWarpDrive,
        )
        .with_group(bindings::BindingGroup::Navigation.as_str())
        .with_context_predicate(id!("Workspace") & id!(flags::ENABLE_WARP_DRIVE))
        .with_mac_key_binding("ctrl-4")
        .with_linux_or_windows_key_binding("alt-4"),
        EditableBinding::new(
            TOGGLE_PROJECT_EXPLORER_BINDING_NAME,
            BindingDescription::new("切换项目浏览器")
                .with_custom_description(bindings::MAC_MENUS_CONTEXT, "项目浏览器"),
            WorkspaceAction::ToggleProjectExplorer,
        )
        .with_context_predicate(id!("Workspace") & id!(flags::SHOW_PROJECT_EXPLORER)),
        EditableBinding::new(
            OPEN_GLOBAL_SEARCH_BINDING_NAME,
            BindingDescription::new("打开全局搜索")
                .with_custom_description(bindings::MAC_MENUS_CONTEXT, "全局搜索"),
            WorkspaceAction::OpenGlobalSearch,
        )
        .with_context_predicate(id!("Workspace") & id!(flags::SHOW_GLOBAL_SEARCH))
        .with_mac_key_binding("cmd-shift-F")
        // we use alt because we use ctrl-shift-f for find because ctrl-f needs to be reserved for the shell
        .with_linux_or_windows_key_binding("alt-shift-F"),
        EditableBinding::new(
            TOGGLE_WARP_DRIVE_BINDING_NAME,
            BindingDescription::new("切换 Warp Drive")
                .with_custom_description(bindings::MAC_MENUS_CONTEXT, "Warp Drive"),
            WorkspaceAction::ToggleWarpDrive,
        )
        .with_context_predicate(id!("Workspace") & id!(flags::ENABLE_WARP_DRIVE)),
        EditableBinding::new(
            TOGGLE_CONVERSATION_LIST_VIEW_BINDING_NAME,
            BindingDescription::new("切换智能体对话列表")
                .with_custom_description(bindings::MAC_MENUS_CONTEXT, "智能体对话列表"),
            WorkspaceAction::ToggleConversationListView,
        )
        .with_enabled(|| FeatureFlag::AgentViewConversationListView.is_enabled())
        .with_context_predicate(id!("Workspace") & id!(flags::SHOW_CONVERSATION_HISTORY))
        .with_mac_key_binding("cmd-shift-A")
        .with_linux_or_windows_key_binding("ctrl-shift-A")
        .with_group(bindings::BindingGroup::WarpAi.as_str()),
        EditableBinding::new(
            "workspace:close_panel",
            BindingDescription::new("关闭聚焦面板")
                .with_custom_description(bindings::MAC_MENUS_CONTEXT, "关闭聚焦面板"),
            WorkspaceAction::ClosePanel,
        )
        .with_context_predicate(id!("Workspace"))
        .with_custom_action(CustomAction::CloseCurrentSession),
        EditableBinding::new(
            "workspace:toggle_command_palette",
            BindingDescription::new("切换命令面板")
                .with_custom_description(bindings::MAC_MENUS_CONTEXT, "命令面板"),
            WorkspaceAction::TogglePalette {
                mode: PaletteMode::Command,
                source: PaletteSource::Keybinding,
            },
        )
        .with_group(bindings::BindingGroup::Settings.as_str())
        .with_context_predicate(id!("Workspace") & !id!("Workspace_CloudConversationWebViewer"))
        .with_custom_action(CustomAction::CommandPalette),
        EditableBinding::new(
            "workspace:move_tab_left",
            BindingDescription::new("标签页左移")
                .with_dynamic_override(|ctx| uses_vertical_tabs(ctx).then(|| "标签页上移".into())),
            WorkspaceAction::MoveActiveTabLeft,
        )
        .with_group(bindings::BindingGroup::Navigation.as_str())
        .with_context_predicate(
            id!("Workspace")
                & id!("Workspace_MultipleTabs")
                & !id!("Workspace_LeftmostTabActive")
                & !id!("Workspace_PaneDragging"),
        )
        .with_custom_action(CustomAction::MoveTabLeft),
        EditableBinding::new(
            "workspace:move_tab_right",
            BindingDescription::new("标签页右移")
                .with_dynamic_override(|ctx| uses_vertical_tabs(ctx).then(|| "标签页下移".into())),
            WorkspaceAction::MoveActiveTabRight,
        )
        .with_group(bindings::BindingGroup::Navigation.as_str())
        .with_context_predicate(
            id!("Workspace")
                & id!("Workspace_MultipleTabs")
                & !id!("Workspace_RightmostTabActive")
                & !id!("Workspace_PaneDragging"),
        )
        .with_custom_action(CustomAction::MoveTabRight),
        EditableBinding::new(
            "workspace:toggle_keybindings_page",
            "切换键盘快捷键",
            WorkspaceAction::ToggleKeybindingsPage,
        )
        .with_group(bindings::BindingGroup::KeyboardShortcuts.as_str())
        .with_context_predicate(id!("Workspace") & !id!("Workspace_TextOpen"))
        .with_custom_action(CustomAction::ToggleKeybindingsPage),
        EditableBinding::new(
            "workspace:show_keybinding_settings",
            "打开快捷键编辑器",
            WorkspaceAction::ConfigureKeybindingSettings {
                keybinding_name: None,
            },
        )
        .with_group(bindings::BindingGroup::KeyboardShortcuts.as_str())
        .with_context_predicate(id!("Workspace"))
        .with_mac_key_binding("cmd-ctrl-k"),
        EditableBinding::new(
            "workspace:toggle_block_snackbar",
            "切换粘性命令标题",
            WorkspaceAction::ToggleBlockSnackbar,
        )
        .with_group(bindings::BindingGroup::Settings.as_str())
        .with_context_predicate(id!("Workspace")),
    ]);

    // TODO(PLAT-113): Support a11y on non-MacOS platforms
    if cfg!(target_os = "macos") {
        app.register_editable_bindings([
            EditableBinding::new(
                "workspace:set_a11y_concise_verbosity_level",
                "[无障碍] 设置简洁播报",
                WorkspaceAction::SetA11yVerbosityLevel(AccessibilityVerbosity::Concise),
            )
            .with_context_predicate(id!("Workspace"))
            .with_key_binding("cmdorctrl-alt-c"),
            EditableBinding::new(
                "workspace:set_a11y_verbose_verbosity_level",
                "[无障碍] 设置详细播报",
                WorkspaceAction::SetA11yVerbosityLevel(AccessibilityVerbosity::Verbose),
            )
            .with_context_predicate(id!("Workspace"))
            .with_key_binding("cmdorctrl-alt-v"),
        ]);
    }

    app.register_editable_bindings([EditableBinding::new(
        "workspace:rename_active_tab",
        "重命名当前标签页",
        WorkspaceAction::RenameActiveTab,
    )
    .with_group(bindings::BindingGroup::Settings.as_str())
    .with_custom_action(CustomAction::RenameTab)
    .with_context_predicate(id!("Workspace"))]);

    // Pane rename — same shape as RenameActiveTab but acts on the focused pane
    // in the active tab. Ships with no default keybinding so it surfaces in
    // Settings → Keyboard shortcuts as remappable; resolves issue #9351, where
    // the action existed only in the right-click context menu and was not
    // reachable via the binding registry.
    app.register_editable_bindings([EditableBinding::new(
        "workspace:rename_active_pane",
        "重命名当前窗格",
        WorkspaceAction::RenameActivePane,
    )
    .with_group(bindings::BindingGroup::Settings.as_str())
    .with_context_predicate(id!("Workspace"))]);

    app.register_editable_bindings([EditableBinding::new(
        "workspace:cycle_active_tab_color",
        "循环切换当前标签页颜色",
        WorkspaceAction::CycleActiveTabColor,
    )
    .with_group(bindings::BindingGroup::Settings.as_str())
    .with_context_predicate(id!("Workspace"))]);

    // Tab grouping bindings (keyless by default; gated on `GroupedTabs`).
    app.register_editable_bindings([
        EditableBinding::new(
            "workspace:new_tab_group",
            "新建标签页组",
            // Reuse the new-session dropdown's action, not a dedicated variant.
            WorkspaceAction::SelectNewSessionMenuItem(NewSessionMenuItem::CreateNewTabGroup),
        )
        .with_enabled(|| FeatureFlag::GroupedTabs.is_enabled())
        .with_group(bindings::BindingGroup::Navigation.as_str())
        .with_context_predicate(id!("Workspace") & !id!("Workspace_PaneDragging")),
        EditableBinding::new(
            "workspace:new_tab_group_from_active_or_selected_tabs",
            "从当前或选中标签页创建标签页组",
            WorkspaceAction::NewTabGroupFromActiveOrSelectedTabs,
        )
        .with_enabled(|| FeatureFlag::GroupedTabs.is_enabled())
        .with_group(bindings::BindingGroup::Navigation.as_str())
        .with_context_predicate(id!("Workspace") & !id!("Workspace_PaneDragging")),
        // Gated on `Workspace_ActiveOrSelectedTabsInGroup`: offered only when
        // there's an unambiguous group to leave — a single-group multi-selection,
        // or (with no selection) a grouped active tab. Mixed selections aren't
        // offered, matching the multi-tab right-click menu.
        EditableBinding::new(
            "workspace:remove_active_or_selected_tabs_from_group",
            "从组中移除当前或选中标签页",
            WorkspaceAction::RemoveActiveOrSelectedTabsFromGroup,
        )
        .with_enabled(|| FeatureFlag::GroupedTabs.is_enabled())
        .with_group(bindings::BindingGroup::Navigation.as_str())
        .with_context_predicate(
            id!("Workspace")
                & id!("Workspace_ActiveOrSelectedTabsInGroup")
                & !id!("Workspace_PaneDragging"),
        ),
    ]);

    // Tab/group pinning bindings (keyless by default; gated on `PinnedTabs`).
    // Pin/unpin are split into separate entries so the palette label tracks
    // the active tab/group's current state.
    app.register_editable_bindings([
        EditableBinding::new(
            "workspace:pin_active_tab",
            "固定当前标签页",
            WorkspaceAction::PinActiveTab,
        )
        .with_enabled(|| FeatureFlag::PinnedTabs.is_enabled())
        .with_group(bindings::BindingGroup::Navigation.as_str())
        .with_context_predicate(
            id!("Workspace") & !id!("Workspace_ActiveTabPinned") & !id!("Workspace_PaneDragging"),
        ),
        EditableBinding::new(
            "workspace:unpin_active_tab",
            "取消固定当前标签页",
            WorkspaceAction::UnpinActiveTab,
        )
        .with_enabled(|| FeatureFlag::PinnedTabs.is_enabled())
        .with_group(bindings::BindingGroup::Navigation.as_str())
        .with_context_predicate(
            id!("Workspace") & id!("Workspace_ActiveTabPinned") & !id!("Workspace_PaneDragging"),
        ),
        EditableBinding::new(
            "workspace:pin_active_tab_group",
            "固定当前标签页组",
            WorkspaceAction::PinActiveTabGroup,
        )
        .with_enabled(|| {
            FeatureFlag::PinnedTabs.is_enabled() && FeatureFlag::GroupedTabs.is_enabled()
        })
        .with_group(bindings::BindingGroup::Navigation.as_str())
        .with_context_predicate(
            id!("Workspace")
                & id!("Workspace_ActiveTabInGroup")
                & !id!("Workspace_ActiveTabGroupPinned")
                & !id!("Workspace_PaneDragging"),
        ),
        EditableBinding::new(
            "workspace:unpin_active_tab_group",
            "取消固定当前标签页组",
            WorkspaceAction::UnpinActiveTabGroup,
        )
        .with_enabled(|| {
            FeatureFlag::PinnedTabs.is_enabled() && FeatureFlag::GroupedTabs.is_enabled()
        })
        .with_group(bindings::BindingGroup::Navigation.as_str())
        .with_context_predicate(
            id!("Workspace")
                & id!("Workspace_ActiveTabInGroup")
                & id!("Workspace_ActiveTabGroupPinned")
                & !id!("Workspace_PaneDragging"),
        ),
    ]);

    app.register_editable_bindings([
        EditableBinding::new(
            "workspace:terminate_app",
            "退出 tzWarp",
            WorkspaceAction::TerminateApp,
        )
        .with_context_predicate(id!("Workspace"))
        .with_group(bindings::BindingGroup::Close.as_str())
        .with_enabled(|| ContextFlag::CloseWindow.is_enabled()),
        EditableBinding::new(
            "workspace:close_window",
            BindingDescription::new("关闭窗口")
                .with_custom_description(bindings::MAC_MENUS_CONTEXT, "关闭窗口"),
            WorkspaceAction::CloseWindow,
        )
        .with_mac_key_binding("cmd-shift-W")
        .with_context_predicate(id!("Workspace"))
        .with_group(bindings::BindingGroup::Close.as_str())
        .with_custom_action(CustomAction::CloseWindow)
        .with_enabled(|| ContextFlag::CloseWindow.is_enabled()),
        EditableBinding::new(
            "workspace:close_active_tab",
            "关闭当前标签页",
            WorkspaceAction::CloseActiveTab,
        )
        .with_custom_action(CustomAction::CloseTab)
        .with_group(bindings::BindingGroup::Close.as_str())
        .with_context_predicate(
            id!("Workspace") & (id!("Workspace_CloseWindow") | id!("Workspace_MultipleTabs")),
        ),
        EditableBinding::new(
            "workspace:close_other_tabs",
            "关闭其他标签页",
            WorkspaceAction::CloseNonActiveTabs,
        )
        .with_custom_action(CustomAction::CloseOtherTabs)
        .with_group(bindings::BindingGroup::Close.as_str())
        .with_context_predicate(id!("Workspace")),
        EditableBinding::new(
            "workspace:close_tabs_right_active_tab",
            BindingDescription::new("关闭右侧标签页").with_dynamic_override(|ctx| {
                uses_vertical_tabs(ctx).then(|| "关闭下方标签页".into())
            }),
            WorkspaceAction::CloseTabsRightActiveTab,
        )
        .with_group(bindings::BindingGroup::Close.as_str())
        .with_custom_action(CustomAction::CloseTabsRight)
        .with_context_predicate(id!("Workspace")),
        // We have two actions depending on the current state
        // (i.e. whether notifications are already on or off).
        EditableBinding::new(
            "workspace:toggle_notifications_on",
            "开启通知",
            WorkspaceAction::ToggleNotifications,
        )
        .with_group(bindings::BindingGroup::Notifications.as_str())
        .with_context_predicate(id!("Workspace") & !id!("Notifications_Enabled")),
        EditableBinding::new(
            "workspace:toggle_notifications_off",
            "关闭通知",
            WorkspaceAction::ToggleNotifications,
        )
        .with_group(bindings::BindingGroup::Notifications.as_str())
        .with_context_predicate(id!("Workspace") & id!("Notifications_Enabled")),
        EditableBinding::new(
            "workspace:toggle_navigation_palette",
            BindingDescription::new("切换导航面板")
                .with_custom_description(bindings::MAC_MENUS_CONTEXT, "导航面板"),
            WorkspaceAction::TogglePalette {
                mode: PaletteMode::Navigation,
                source: PaletteSource::Keybinding,
            },
        )
        .with_group(bindings::BindingGroup::Navigation.as_str())
        .with_context_predicate(id!("Workspace"))
        .with_custom_action(CustomAction::NavigationPalette),
        EditableBinding::new(
            "workspace:toggle_launch_config_palette",
            "启动配置面板",
            WorkspaceAction::TogglePalette {
                mode: PaletteMode::LaunchConfig,
                source: PaletteSource::Keybinding,
            },
        )
        .with_context_predicate(id!("Workspace"))
        .with_custom_action(CustomAction::LaunchConfigPalette)
        .with_enabled(|| ContextFlag::LaunchConfigurations.is_enabled()),
        EditableBinding::new(
            "workspace:toggle_files_palette",
            "切换文件面板",
            WorkspaceAction::TogglePalette {
                mode: PaletteMode::Files,
                source: PaletteSource::Keybinding,
            },
        )
        .with_context_predicate(id!("Workspace") & !id!("Workspace_ViewOnlySharedSession"))
        .with_custom_action(CustomAction::FilesPalette),
        EditableBinding::new(
            "workspace:open_launch_config_save_modal",
            "保存新的启动配置",
            WorkspaceAction::OpenLaunchConfigSaveModal,
        )
        .with_context_predicate(id!("Workspace"))
        .with_custom_action(CustomAction::SaveCurrentConfig)
        .with_enabled(|| ContextFlag::LaunchConfigurations.is_enabled()),
        #[cfg(not(feature = "team_relay"))]
        EditableBinding::new(
            // If you rename this name, please update the name in command_palette/action/data_source.rs
            "workspace:search_drive",
            "搜索 Warp Drive",
            WorkspaceAction::OpenPalette {
                mode: PaletteMode::WarpDrive,
                source: PaletteSource::Keybinding,
                query: None,
            },
        )
        .with_context_predicate(id!("Workspace"))
        .with_custom_action(CustomAction::SearchDrive),
    ]);

    if FeatureFlag::Autoupdate.is_enabled() {
        app.register_editable_bindings([
            EditableBinding::new(
                "workspace:update_and_relaunch",
                "安装更新并重新启动",
                // TODO(vorporeal): I wonder if we should change wording here?
                WorkspaceAction::ApplyUpdate,
            )
            .with_group(bindings::BindingGroup::AutoUpdate.as_str())
            .with_context_predicate(id!("Workspace") & id!("AutoupdateState_UpdateReady"))
            .with_enabled(|| ContextFlag::PromptForVersionUpdates.is_enabled()),
            EditableBinding::new(
                "workspace:check_for_updates",
                "检查更新",
                WorkspaceAction::CheckForUpdate,
            )
            .with_group(bindings::BindingGroup::AutoUpdate.as_str())
            .with_context_predicate(id!("Workspace") & !id!("AutoupdateState_UpdateReady"))
            .with_enabled(|| ContextFlag::PromptForVersionUpdates.is_enabled()),
        ]);
    }

    app.register_editable_bindings([EditableBinding::new(
        "workspace:log_out",
        "退出登录",
        WorkspaceAction::LogOut,
    )
    .with_group(bindings::BindingGroup::Settings.as_str())
    .with_context_predicate(id!("Workspace") & !id!("IsAnonymousUser"))]);

    if !FeatureFlag::AvatarInTabBar.is_enabled() {
        app.register_editable_bindings([EditableBinding::new(
            "workspace:toggle_resource_center",
            "切换资源中心",
            WorkspaceAction::ToggleResourceCenter,
        )
        .with_group(bindings::BindingGroup::Navigation.as_str())
        .with_context_predicate(id!("Workspace"))
        .with_custom_action(CustomAction::ToggleResourceCenter)]);
    }

    if cfg!(not(target_family = "wasm")) {
        #[cfg(not(feature = "team_relay"))]
        app.register_editable_bindings([EditableBinding::new(
            "workspace:export_all_warp_drive_objects",
            "导出全部 Warp Drive 对象",
            WorkspaceAction::ExportAllWarpDriveObjects,
        )
        .with_group(bindings::BindingGroup::Settings.as_str())
        .with_context_predicate(id!("Workspace") & id!(flags::ENABLE_WARP_DRIVE))]);
    }

    // Oz and Warp Control CLI install/uninstall actions (macOS only)
    #[cfg(target_os = "macos")]
    {
        app.register_editable_bindings([
            #[cfg(not(feature = "team_relay"))]
            EditableBinding::new(
                "workspace:install_cli",
                "全局安装 Oz CLI（可在应用外使用）",
                WorkspaceAction::InstallOz,
            )
            .with_group(bindings::BindingGroup::Settings.as_str())
            .with_context_predicate(id!("Workspace")),
            #[cfg(not(feature = "team_relay"))]
            EditableBinding::new(
                "workspace:uninstall_cli",
                "撤销全局 Oz CLI 安装（应用内仍可用）",
                WorkspaceAction::UninstallOz,
            )
            .with_group(bindings::BindingGroup::Settings.as_str())
            .with_context_predicate(id!("Workspace")),
        ]);
        if FeatureFlag::WarpControlCli.is_enabled() {
            app.register_editable_bindings([
                #[cfg(not(feature = "team_relay"))]
                EditableBinding::new(
                    "workspace:install_warpctrl",
                    "全局安装控制 CLI（可在应用外使用）",
                    WorkspaceAction::InstallWarpctrl,
                )
                .with_group(bindings::BindingGroup::Settings.as_str())
                .with_context_predicate(id!("Workspace")),
                #[cfg(not(feature = "team_relay"))]
                EditableBinding::new(
                    "workspace:uninstall_warpctrl",
                    "撤销全局控制 CLI 安装（应用内仍可用）",
                    WorkspaceAction::UninstallWarpctrl,
                )
                .with_group(bindings::BindingGroup::Settings.as_str())
                .with_context_predicate(id!("Workspace")),
            ]);
        }
    }

    if FeatureFlag::Changelog.is_enabled() {
        app.register_editable_bindings([
            // Always show the "View latest changelog" action in the command palette,
            // but without a keybinding when the update toast is not visible.
            EditableBinding::new(
                "workspace:view_changelog",
                "查看最新更新日志",
                WorkspaceAction::ViewLatestChangelog,
            )
            .with_context_predicate(id!("Workspace") & !id!("UpdateToastVisible"))
            .with_group(bindings::BindingGroup::Settings.as_str())
            // Note that while the changelog resides in WarpEssentials, we should gate access to
            // the changelog based on whether WarpEssentials is an available view.
            .with_enabled(|| ContextFlag::WarpEssentials.is_enabled()),
            // When the update toast is visible, register the keybinding as well.
            EditableBinding::new(
                "workspace:view_changelog",
                "查看最新更新日志",
                WorkspaceAction::ViewLatestChangelog,
            )
            .with_context_predicate(id!("Workspace") & id!("UpdateToastVisible"))
            .with_group(bindings::BindingGroup::Settings.as_str())
            .with_custom_action(CustomAction::ViewChangelog)
            .with_linux_or_windows_key_binding(format!("alt-{}", cmd_or_ctrl_shift("o")))
            .with_enabled(|| ContextFlag::WarpEssentials.is_enabled()),
        ]);
    }

    // We use the same binding name for the AI Assistant and block list AI to preserve custom
    // keybindings between them.
    app.register_editable_bindings([
        EditableBinding::new(
            "workspace:toggle_ai_assistant",
            *NEW_AGENT_PANE_LABEL,
            WorkspaceAction::NewPaneInAgentMode {
                entrypoint: AgentModeEntrypoint::NewPaneBinding,
                zero_state_prompt_suggestion_type: None,
            },
        )
        .with_enabled(|| FeatureFlag::AgentMode.is_enabled())
        .with_context_predicate(id!("Workspace") & id!(flags::IS_ANY_AI_ENABLED))
        .with_group(bindings::BindingGroup::WarpAi.as_str())
        .with_custom_action(CustomAction::NewAgentModePane),
        EditableBinding::new(
            "workspace:toggle_ai_assistant",
            "切换 AI",
            WorkspaceAction::ToggleAIAssistant,
        )
        .with_enabled(|| !FeatureFlag::AgentMode.is_enabled())
        .with_context_predicate(id!("Workspace") & id!(flags::IS_ANY_AI_ENABLED))
        .with_group(bindings::BindingGroup::WarpAi.as_str())
        // We use the same custom action as AM so that we don't have
        // two mac menu items for AM vs Warp AI since they are mutually exclusive.
        .with_custom_action(CustomAction::NewAgentModePane),
    ]);

    app.register_editable_bindings([
        #[cfg(not(feature = "team_relay"))]
        EditableBinding::new(
            "workspace:create_team_env_vars",
            BindingDescription::new("新建团队环境变量")
                .with_custom_description(bindings::MAC_MENUS_CONTEXT, "新建团队环境变量"),
            WorkspaceAction::CreateTeamEnvVarCollection,
        )
        .with_custom_action(CustomAction::NewTeamEnvVars)
        .with_context_predicate(
            id!("Workspace")
                & id!(flags::ENABLE_WARP_DRIVE)
                & id!("WarpDrive_BelongsToTeam")
                & id!("IsOnline"),
        )
        .with_group(bindings::BindingGroup::EnvVarCollection.as_str()),
        #[cfg(not(feature = "team_relay"))]
        EditableBinding::new(
            "workspace:create_personal_env_vars",
            BindingDescription::new("新建个人环境变量")
                .with_custom_description(bindings::MAC_MENUS_CONTEXT, "新建个人环境变量"),
            WorkspaceAction::CreatePersonalEnvVarCollection,
        )
        .with_group(bindings::BindingGroup::EnvVarCollection.as_str())
        .with_custom_action(CustomAction::NewPersonalEnvVars)
        .with_context_predicate(id!("Workspace") & id!(flags::ENABLE_WARP_DRIVE)),
        #[cfg(not(feature = "team_relay"))]
        EditableBinding::new(
            "workspace:create_personal_ai_prompt",
            BindingDescription::new("新建个人提示词")
                .with_custom_description(bindings::MAC_MENUS_CONTEXT, "新建个人提示词"),
            WorkspaceAction::CreatePersonalAIPrompt,
        )
        .with_group(bindings::BindingGroup::WarpAi.as_str())
        .with_custom_action(CustomAction::NewPersonalAIPrompt)
        .with_context_predicate(
            id!("Workspace") & id!(flags::ENABLE_WARP_DRIVE) & id!(flags::IS_ANY_AI_ENABLED),
        ),
        #[cfg(not(feature = "team_relay"))]
        EditableBinding::new(
            "workspace:create_team_ai_prompt",
            BindingDescription::new("新建团队提示词")
                .with_custom_description(bindings::MAC_MENUS_CONTEXT, "新建团队提示词"),
            WorkspaceAction::CreateTeamAIPrompt,
        )
        .with_group(bindings::BindingGroup::WarpAi.as_str())
        .with_custom_action(CustomAction::NewTeamAIPrompt)
        .with_context_predicate(
            id!("Workspace")
                & id!(flags::ENABLE_WARP_DRIVE)
                & id!("WarpDrive_BelongsToTeam")
                & id!("IsOnline")
                & id!(flags::IS_ANY_AI_ENABLED),
        ),
    ]);

    app.register_editable_bindings([
        EditableBinding::new(
            "workspace:shift_focus_left",
            "切换焦点到左侧面板",
            WorkspaceAction::FocusLeftPanel,
        )
        .with_context_predicate(id!("Workspace"))
        .with_key_binding("cmdorctrl-shift-("),
        EditableBinding::new(
            "workspace:shift_focus_right",
            "切换焦点到右侧面板",
            WorkspaceAction::FocusRightPanel,
        )
        .with_context_predicate(id!("Workspace"))
        .with_key_binding("cmdorctrl-shift-)"),
    ]);

    app.register_editable_bindings([
        #[cfg(not(feature = "team_relay"))]
        EditableBinding::new(
            "workspace:import_to_personal_drive",
            "导入到个人 Drive",
            WorkspaceAction::ImportToPersonalDrive,
        )
        .with_context_predicate(id!("Workspace") & id!(flags::ENABLE_WARP_DRIVE)),
        #[cfg(not(feature = "team_relay"))]
        EditableBinding::new(
            "workspace:import_to_team_drive",
            "导入到团队 Drive",
            WorkspaceAction::ImportToTeamDrive,
        )
        .with_context_predicate(
            id!("Workspace") & id!(flags::ENABLE_WARP_DRIVE) & id!("WarpDrive_BelongsToTeam"),
        ),
    ]);

    // Register a debug-only action for writing the user's access token to the system clipboard
    // to aid debugging and development.
    #[cfg(not(feature = "skip_login"))]
    if ChannelState::enable_debug_features() {
        app.register_editable_bindings([EditableBinding::new(
            "workspace:copy_access_token_to_clipboard",
            "复制访问令牌到剪贴板",
            WorkspaceAction::CopyAccessTokenToClipboard,
        )
        .with_context_predicate(id!("Workspace"))]);
    }

    app.register_editable_bindings([
        EditableBinding::new(
            "workspace:copy_current_path",
            BindingDescription::new("复制当前路径")
                .with_custom_description(bindings::MAC_MENUS_CONTEXT, "复制当前路径"),
            WorkspaceAction::CopyCurrentPath,
        )
        .with_context_predicate(id!("Workspace")),
        EditableBinding::new(
            "workspace:open_repository",
            BindingDescription::new("打开仓库")
                .with_custom_description(bindings::MAC_MENUS_CONTEXT, "打开仓库"),
            WorkspaceAction::OpenRepository { path: None },
        )
        .with_context_predicate(id!("Workspace"))
        .with_custom_action(CustomAction::OpenRepository)
        .with_group(bindings::BindingGroup::Folders.as_str()),
        EditableBinding::new(
            "workspace:open_ai_fact_collection",
            BindingDescription::new("打开 AI 规则")
                .with_custom_description(bindings::MAC_MENUS_CONTEXT, "打开 AI 规则"),
            WorkspaceAction::OpenAIFactCollection,
        )
        .with_enabled(|| FeatureFlag::AIRules.is_enabled())
        .with_custom_action(CustomAction::OpenAIFactCollection)
        .with_context_predicate(id!("Workspace") & id!(flags::IS_ANY_AI_ENABLED))
        .with_group(bindings::BindingGroup::WarpAi.as_str()),
    ]);

    #[cfg(not(feature = "team_relay"))]
    app.register_editable_bindings([EditableBinding::new(
        "workspace:open_mcp_servers",
        BindingDescription::new("打开 MCP 服务器")
            .with_custom_description(bindings::MAC_MENUS_CONTEXT, "打开 MCP 服务器"),
        WorkspaceAction::OpenMCPServerCollection,
    )
    .with_enabled(|| {
        FeatureFlag::McpServer.is_enabled() && ContextFlag::ShowMCPServers.is_enabled()
    })
    .with_custom_action(CustomAction::OpenMCPServerCollection)
    .with_context_predicate(id!("Workspace") & id!(flags::IS_ANY_AI_ENABLED))
    .with_group(bindings::BindingGroup::WarpAi.as_str())]);

    app.register_editable_bindings([EditableBinding::new(
        "workspace:jump_to_latest_toast",
        "跳转到最新智能体任务",
        WorkspaceAction::JumpToLatestToast,
    )
    .with_enabled(|| FeatureFlag::AgentMode.is_enabled())
    .with_context_predicate(id!("Workspace") & id!(flags::IS_ANY_AI_ENABLED))
    .with_mac_key_binding("cmd-shift-G")
    .with_linux_or_windows_key_binding("ctrl-shift-G")
    .with_group(bindings::BindingGroup::WarpAi.as_str())]);

    app.register_editable_bindings([EditableBinding::new(
        TOGGLE_NOTIFICATION_MAILBOX_BINDING_NAME,
        "切换通知收件箱",
        WorkspaceAction::ToggleNotificationMailbox { select_first: true },
    )
    .with_enabled(|| {
        !cfg!(feature = "team_relay") && FeatureFlag::HOANotifications.is_enabled()
    })
    .with_context_predicate(id!("Workspace"))
    .with_mac_key_binding("cmd-shift-U")
    .with_linux_or_windows_key_binding("ctrl-shift-U")
    .with_group(bindings::BindingGroup::WarpAi.as_str())]);

    add_open_setting_pages_as_editable_binding(app);
    add_overflow_menu_items_as_editable_binding(app);

    app.register_editable_bindings([EditableBinding::new(
        "workspace:toggle_agent_management_view",
        "切换智能体管理视图",
        WorkspaceAction::ToggleAgentManagementView,
    )
    .with_enabled(|| {
        !cfg!(feature = "team_relay") && FeatureFlag::AgentManagementView.is_enabled()
    })
    .with_context_predicate(id!("Workspace") & id!(flags::IS_ANY_AI_ENABLED))
    .with_mac_key_binding("cmd-shift-M")
    .with_linux_or_windows_key_binding("ctrl-shift-M")
    .with_group(bindings::BindingGroup::WarpAi.as_str())]);
}

fn add_open_setting_pages_as_editable_binding(app: &mut AppContext) {
    use warpui::keymap::macros::*;

    // Add the ability to open setting modals to the command palette.
    app.register_editable_bindings([
        EditableBinding::new(
            "workspace:show_settings",
            BindingDescription::new("打开设置")
                .with_custom_description(bindings::MAC_MENUS_CONTEXT, "设置"),
            WorkspaceAction::ShowSettings,
        )
        .with_context_predicate(id!("Workspace"))
        .with_group(bindings::BindingGroup::Settings.as_str())
        .with_custom_action(CustomAction::ShowSettings),
        // 小团队模式不暴露 Warp 账户设置入口。
        #[cfg(not(feature = "team_relay"))]
        EditableBinding::new(
            "workspace:show_settings_account_page",
            "打开设置：账户",
            WorkspaceAction::ShowSettingsPage(SettingsSection::Account),
        )
        .with_context_predicate(id!("Workspace"))
        .with_group(bindings::BindingGroup::Settings.as_str())
        .with_custom_action(CustomAction::ShowAccount),
        EditableBinding::new(
            "workspace:show_settings_appearance_page",
            BindingDescription::new("打开设置：外观")
                .with_custom_description(bindings::MAC_MENUS_CONTEXT, "外观…"),
            WorkspaceAction::ShowSettingsPage(SettingsSection::Appearance),
        )
        .with_group(bindings::BindingGroup::Settings.as_str())
        .with_context_predicate(id!("Workspace"))
        .with_custom_action(CustomAction::ShowAppearance),
        EditableBinding::new(
            "workspace:show_settings_features_page",
            "打开设置：功能",
            WorkspaceAction::ShowSettingsPage(SettingsSection::Features),
        )
        .with_group(bindings::BindingGroup::Settings.as_str())
        .with_context_predicate(id!("Workspace")),
        #[cfg(not(feature = "team_relay"))]
        EditableBinding::new(
            "workspace:show_settings_shared_blocks_page",
            BindingDescription::new("打开设置：共享代码块")
                .with_custom_description(bindings::MAC_MENUS_CONTEXT, "查看共享代码块…"),
            WorkspaceAction::ShowSettingsPage(SettingsSection::SharedBlocks),
        )
        .with_group(bindings::BindingGroup::Settings.as_str())
        .with_context_predicate(id!("Workspace"))
        .with_custom_action(CustomAction::ViewSharedBlocks),
        EditableBinding::new(
            "workspace:show_settings_keyboard_shortcuts_page",
            BindingDescription::new("打开设置：键盘快捷键")
                .with_custom_description(bindings::MAC_MENUS_CONTEXT, "配置键盘快捷键…"),
            WorkspaceAction::ShowSettingsPage(SettingsSection::Keybindings),
        )
        .with_group(bindings::BindingGroup::KeyboardShortcuts.as_str())
        .with_context_predicate(id!("Workspace"))
        .with_custom_action(CustomAction::ConfigureKeybindings),
        EditableBinding::new(
            "workspace:show_settings_about_page",
            BindingDescription::new("打开设置：关于")
                .with_custom_description(bindings::MAC_MENUS_CONTEXT, "关于 tzWarp"),
            WorkspaceAction::ShowSettingsPage(SettingsSection::About),
        )
        .with_group(bindings::BindingGroup::Settings.as_str())
        .with_context_predicate(id!("Workspace"))
        .with_custom_action(CustomAction::ShowAboutWarp),
        #[cfg(not(feature = "team_relay"))]
        EditableBinding::new(
            "workspace:show_settings_teams_page",
            BindingDescription::new("打开设置：团队")
                .with_custom_description(bindings::MAC_MENUS_CONTEXT, "打开团队设置"),
            WorkspaceAction::ShowSettingsPage(SettingsSection::Teams),
        )
        .with_group(bindings::BindingGroup::Settings.as_str())
        .with_custom_action(CustomAction::OpenTeamSettings)
        .with_context_predicate(id!("Workspace")),
        #[cfg(not(feature = "team_relay"))]
        EditableBinding::new(
            "workspace:show_settings_privacy_page",
            BindingDescription::new("打开设置：隐私"),
            WorkspaceAction::ShowSettingsPage(SettingsSection::Privacy),
        )
        .with_group(bindings::BindingGroup::Settings.as_str())
        .with_context_predicate(id!("Workspace")),
        #[cfg(not(feature = "team_relay"))]
        EditableBinding::new(
            "workspace:show_settings_warpify_page",
            BindingDescription::new("打开设置：Warpify")
                .with_custom_description(bindings::MAC_MENUS_CONTEXT, "配置 Warpify…"),
            WorkspaceAction::ShowSettingsPage(SettingsSection::Warpify),
        )
        .with_group(bindings::BindingGroup::Settings.as_str())
        .with_context_predicate(id!("Workspace")),
        EditableBinding::new(
            "workspace:show_ai_settings_page",
            BindingDescription::new("打开设置：AI"),
            WorkspaceAction::ShowSettingsPage(SettingsSection::WarpAgent),
        )
        .with_enabled(|| FeatureFlag::AgentMode.is_enabled())
        .with_group(bindings::BindingGroup::Settings.as_str())
        .with_context_predicate(id!("Workspace")),
        #[cfg(not(feature = "team_relay"))]
        EditableBinding::new(
            "workspace:show_settings_billing_and_usage_page",
            BindingDescription::new("打开设置：账单与用量"),
            WorkspaceAction::ShowSettingsPage(SettingsSection::BillingAndUsage),
        )
        .with_group(bindings::BindingGroup::Settings.as_str())
        .with_context_predicate(id!("Workspace")),
        #[cfg(not(feature = "team_relay"))]
        EditableBinding::new(
            "workspace:show_settings_code_page",
            BindingDescription::new("打开设置：代码"),
            WorkspaceAction::ShowSettingsPage(SettingsSection::CodeIndexing),
        )
        .with_group(bindings::BindingGroup::Settings.as_str())
        .with_context_predicate(id!("Workspace")),
        #[cfg(not(feature = "team_relay"))]
        EditableBinding::new(
            "workspace:show_settings_referrals_page",
            BindingDescription::new("打开设置：推荐"),
            WorkspaceAction::ShowSettingsPage(SettingsSection::Referrals),
        )
        .with_group(bindings::BindingGroup::Settings.as_str())
        .with_context_predicate(id!("Workspace")),
        #[cfg(not(feature = "team_relay"))]
        EditableBinding::new(
            "workspace:show_settings_environments_page",
            BindingDescription::new("打开设置：环境"),
            WorkspaceAction::ShowSettingsPage(SettingsSection::CloudEnvironments),
        )
        .with_group(bindings::BindingGroup::Settings.as_str())
        .with_context_predicate(id!("Workspace")),
        #[cfg(not(feature = "team_relay"))]
        EditableBinding::new(
            "workspace:show_mcp_servers_settings_page",
            BindingDescription::new("打开设置：MCP 服务器"),
            WorkspaceAction::ShowSettingsPage(SettingsSection::MCPServers),
        )
        .with_group(bindings::BindingGroup::Settings.as_str())
        .with_context_predicate(id!("Workspace")),
        EditableBinding::new(
            "workspace:open_settings_file",
            "打开设置文件",
            WorkspaceAction::OpenSettingsFile,
        )
        .with_enabled(|| FeatureFlag::SettingsFile.is_enabled() && cfg!(feature = "local_fs"))
        .with_group(bindings::BindingGroup::Settings.as_str())
        .with_context_predicate(id!("Workspace")),
    ]);
}

fn add_overflow_menu_items_as_editable_binding(app: &mut AppContext) {
    use warpui::keymap::macros::*;

    // Add the ability to open all overflow menu items to the command palette.
    app.register_editable_bindings([
        #[cfg(not(feature = "team_relay"))]
        EditableBinding::new(
            "workspace:show_invite_modal",
            "邀请成员…",
            WorkspaceAction::ShowReferralSettingsPage,
        )
        .with_context_predicate(id!("Workspace"))
        .with_custom_action(CustomAction::ReferAFriend),
        #[cfg(not(feature = "team_relay"))]
        EditableBinding::new(
            "workspace:link_to_slack",
            "加入 Slack 社区（打开外部链接）",
            WorkspaceAction::JoinSlack,
        )
        .with_context_predicate(id!("Workspace")),
        EditableBinding::new(
            "workspace:link_to_user_docs",
            "查看用户文档（打开外部链接）",
            WorkspaceAction::ViewUserDocs,
        )
        .with_context_predicate(id!("Workspace")),
        #[cfg(not(feature = "team_relay"))]
        EditableBinding::new(
            "workspace:send_feedback",
            BindingDescription::new("发送反馈（打开外部链接）"),
            WorkspaceAction::SendFeedback,
        )
        .with_context_predicate(id!("Workspace")),
        #[cfg(not(target_family = "wasm"))]
        EditableBinding::new("workspace:view_logs", "查看日志", WorkspaceAction::ViewLogs)
            .with_context_predicate(id!("Workspace")),
        #[cfg(not(feature = "team_relay"))]
        EditableBinding::new(
            "workspace:link_to_privacy_policy",
            "查看隐私政策（打开外部链接）",
            WorkspaceAction::ViewPrivacyPolicy,
        )
        .with_context_predicate(id!("Workspace")),
    ]);
}

#[derive(PartialEq, Copy, Clone, Debug)]
pub struct TabBarDropTargetData {
    pub tab_bar_location: TabBarLocation,
}

#[derive(PartialEq, Copy, Clone, Debug)]
pub struct VerticalTabsPaneDropTargetData {
    pub tab_bar_location: TabBarLocation,
}

#[derive(PartialEq, Copy, Clone, Debug, Serialize, Deserialize)]
pub enum TabBarLocation {
    TabIndex(usize),
    AfterTabIndex(usize), // Indicates any area after the tabs in the tab bar, includes the total tab count
}

impl DropTargetData for TabBarDropTargetData {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl DropTargetData for VerticalTabsPaneDropTargetData {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
