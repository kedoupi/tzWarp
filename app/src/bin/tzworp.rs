// On Windows, we don't want to display a console window when the application is running in release
// builds. See https://doc.rust-lang.org/reference/runtime.html#the-windows_subsystem-attribute.
#![cfg_attr(feature = "release_bundle", windows_subsystem = "windows")]

use anyhow::Result;
use warp_core::AppId;
use warp_core::channel::{Channel, ChannelConfig, ChannelState, OzConfig, WarpServerConfig};

/// tzWarp：基于官方 Warp 客户端的团队定制构建。
///
/// - 启用 `team_relay`：AI 直连 `https://tzai.kdp.cool/v1`
/// - 启用 `skip_login`：无需 Warp 账号
/// - 独立 bundle id / 数据目录，可与官方 Warp 并存
fn main() -> Result<()> {
    // 默认隐藏调试命令面板项；需要时 export WARP_DEBUG_FEATURES=1
    if std::env::var_os("WARP_DEBUG_FEATURES").is_none() {
        // SAFETY: single-threaded before any threads spawn.
        unsafe { std::env::set_var("WARP_DEBUG_FEATURES", "0") };
    }

    let mut state = ChannelState::new(
        Channel::Oss,
        ChannelConfig {
            app_id: AppId::new("cool", "kdp", "tzWarp"),
            logfile_name: "tzworp.log".into(),
            server_config: WarpServerConfig::production(),
            oz_config: OzConfig::production(),
            telemetry_config: None,
            crash_reporting_config: None,
            autoupdate_config: None,
            mcp_static_config: None,
        },
    );
    // 产品包不注入 DEBUG_FLAGS；仅当显式 WARP_DEBUG_FEATURES=1 时打开。
    if matches!(
        std::env::var("WARP_DEBUG_FEATURES").as_deref(),
        Ok("1") | Ok("true")
    ) {
        state = state.with_additional_features(warp_core::features::DEBUG_FLAGS);
    }
    ChannelState::set(state);

    warp::run()
}

// If we're not using an external plist, embed the following as the Info.plist.
#[cfg(all(not(feature = "extern_plist"), target_os = "macos"))]
embed_plist::embed_info_plist_bytes!(r#"
    <?xml version="1.0" encoding="UTF-8"?>
    <!DOCTYPE plist PUBLIC "-//Apple Computer//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
    <plist version="1.0">
    <dict>
    <key>CFBundleDevelopmentRegion</key>
    <string>zh_CN</string>
    <key>CFBundleDisplayName</key>
    <string>tzWarp</string>
    <key>CFBundleExecutable</key>
    <string>tzworp</string>
    <key>CFBundleIdentifier</key>
    <string>cool.kdp.tzWarp</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>CFBundleName</key>
    <string>tzWarp</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleIconFile</key>
    <string>AppIcon</string>
    <key>CFBundleIconName</key>
    <string>AppIcon</string>
    <key>CFBundleShortVersionString</key>
    <string>1.0.0</string>
    <key>CFBundleVersion</key>
    <string>1.0.0</string>
    <key>LSApplicationCategoryType</key>
    <string>public.app-category.developer-tools</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>UIDesignRequiresCompatibility</key>
    <true/>
    <key>CFBundleURLTypes</key>
    <array><dict><key>CFBundleURLName</key><string>tzWarp</string><key>CFBundleURLSchemes</key><array><string>tzworp</string></array></dict></array>
    <key>NSHumanReadableCopyright</key>
    <string>© 2026 tzWarp (based on Warp AGPL)</string>
    </dict>
    </plist>
"#.as_bytes());
