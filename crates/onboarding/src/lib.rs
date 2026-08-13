// Onboarding library crate

mod agent_onboarding_view;
pub mod callout;
mod model;
pub mod slides;
pub mod telemetry;

/// The user's intention selected during onboarding slides.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OnboardingIntention {
    Terminal,
    AgentDrivenDevelopment,
}

impl std::fmt::Display for OnboardingIntention {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OnboardingIntention::AgentDrivenDevelopment => write!(f, "agent_driven"),
            OnboardingIntention::Terminal => write!(f, "terminal"),
        }
    }
}

pub use callout::{OnboardingCalloutView, OnboardingKeybindings};

/// User-facing descriptions of the AI features enabled when the agent intention is selected.
/// Shared by the intention slide's agent card checklist and the login slide's
/// skip-login confirmation dialog so the two always stay in sync.
pub const AI_FEATURES: &[&str] = &[
    "使用 tzWarp Agent 与本地可用模型",
    "处理终端错误并给出修复建议",
    "管理长时间运行的命令和 TUI",
    "查看代码差异并向 Agent 发送评论",
    "配合 Claude Code、Codex 等 Agent",
];

/// User-facing names of the Warp Drive features enabled when the terminal
/// intention is selected with Warp Drive turned on. Shared by the login slide's
/// skip-login confirmation dialog so the list stays in sync with any future
/// surfaces that need it.
pub const WARP_DRIVE_FEATURES: &[&str] = &["tzWarp Drive", "会话共享"];

cfg_if::cfg_if! {
    if #[cfg(feature = "bin")] {
        mod telemetry_provider;
        pub use telemetry_provider::MockTelemetryContextProvider;
    }
}

pub mod components;
mod visuals;

/// The default mode for new sessions, chosen during onboarding.
/// Mapped to `DefaultSessionMode` at the application boundary.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SessionDefault {
    #[default]
    Agent,
    Terminal,
}

impl std::fmt::Display for SessionDefault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionDefault::Agent => write!(f, "agent"),
            SessionDefault::Terminal => write!(f, "terminal"),
        }
    }
}

pub use agent_onboarding_view::{AgentOnboardingAction, AgentOnboardingEvent, AgentOnboardingView};
pub use model::{
    ChooseHowToStartExperimentArm, CreditPackOption, CreditPurchaseState, OnboardingAuthState,
    SelectedSettings, UICustomizationSettings,
};
pub use slides::{OfferVariant, ProjectOnboardingSettings};
pub use telemetry::OnboardingEvent;

pub fn init(app: &mut warpui_core::AppContext) {
    agent_onboarding_view::init(app);
    callout::init(app);
}
