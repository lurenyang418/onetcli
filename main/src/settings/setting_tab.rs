use std::path::PathBuf;

use gpui::{div, App, ClickEvent, FontWeight, IntoElement, Keystroke, ParentElement, Styled};
use gpui_component::{
    button::{Button, ButtonVariants as _},
    clipboard::Clipboard,
    h_flex,
    kbd::Kbd,
    v_flex, ActiveTheme, IconName, Sizable, Theme, ThemeMode,
};
use one_core::storage::manager::get_config_dir;
use one_core::utils::auto_save_config::AutoSaveConfig;
use rust_i18n::t;
use serde::{Deserialize, Serialize};
use tracing::{error, info};

// ============================================================================
// 数据库配置
// ============================================================================

/// 数据库打开方式
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum DatabaseOpenMode {
    /// 单库模式：每个数据库单独打开一个标签页
    #[default]
    Single,
    /// 工作区模式：按工作区分组打开，同一工作区的数据库在同一标签页
    Workspace,
}

impl DatabaseOpenMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            DatabaseOpenMode::Single => "single",
            DatabaseOpenMode::Workspace => "workspace",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "workspace" => DatabaseOpenMode::Workspace,
            _ => DatabaseOpenMode::Single,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    #[serde(default)]
    pub locale: String,
    #[serde(default)]
    pub theme_mode: String,
    #[serde(default)]
    pub auto_switch_theme: bool,
    #[serde(default = "default_font_family")]
    pub font_family: String,
    #[serde(default = "default_font_size")]
    pub font_size: f64,
    #[serde(default = "default_true")]
    pub auto_update: bool,
    #[serde(default)]
    pub database_open_mode: DatabaseOpenMode,
    /// 是否启用SQL查询的自动保存功能
    #[serde(default = "default_true")]
    pub enable_sql_auto_save: bool,
    /// SQL查询自动保存的间隔（秒），默认5秒
    #[serde(default = "default_auto_save_interval")]
    pub sql_auto_save_interval: f64,
    /// 启动时是否最大化窗口
    #[serde(default = "default_true")]
    pub start_maximized: bool,
}

fn default_font_family() -> String {
    "Arial".to_string()
}

fn default_font_size() -> f64 {
    14.0
}

fn default_true() -> bool {
    true
}

fn default_auto_save_interval() -> f64 {
    5.0
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            locale: "zh-CN".to_string(),
            theme_mode: "light".to_string(),
            auto_switch_theme: false,
            font_family: default_font_family(),
            font_size: default_font_size(),
            auto_update: true,
            database_open_mode: DatabaseOpenMode::default(),
            enable_sql_auto_save: true,
            sql_auto_save_interval: default_auto_save_interval(),
            start_maximized: true,
        }
    }
}

impl gpui::Global for AppSettings {}

impl AppSettings {
    pub fn global(cx: &App) -> &AppSettings {
        cx.global::<AppSettings>()
    }

    pub fn global_mut(cx: &mut App) -> &mut AppSettings {
        cx.global_mut::<AppSettings>()
    }

    fn config_path() -> Option<PathBuf> {
        get_config_dir().ok().map(|dir| dir.join("settings.json"))
    }

    pub fn load() -> Self {
        let Some(path) = Self::config_path() else {
            return Self::default();
        };

        if !path.exists() {
            return Self::default();
        }

        match std::fs::read_to_string(&path) {
            Ok(content) => match serde_json::from_str(&content) {
                Ok(settings) => {
                    info!("Settings loaded from {:?}", path);
                    settings
                }
                Err(e) => {
                    error!("Failed to parse settings: {}", e);
                    Self::default()
                }
            },
            Err(e) => {
                error!("Failed to read settings file: {}", e);
                Self::default()
            }
        }
    }

    pub fn save(&self) {
        let Some(path) = Self::config_path() else {
            error!("Could not determine config path");
            return;
        };

        if let Some(parent) = path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                error!("Failed to create config directory: {}", e);
                return;
            }
        }

        match serde_json::to_string_pretty(self) {
            Ok(content) => {
                if let Err(e) = std::fs::write(&path, content) {
                    error!("Failed to write settings file: {}", e);
                } else {
                    info!("Settings saved to {:?}", path);
                }
            }
            Err(e) => {
                error!("Failed to serialize settings: {}", e);
            }
        }
    }

    pub fn apply(&self, cx: &mut App) {
        gpui_component::set_locale(&self.locale);

        let mode = if self.theme_mode == "dark" {
            ThemeMode::Dark
        } else {
            ThemeMode::Light
        };
        Theme::global_mut(cx).mode = mode;
        Theme::change(mode, None, cx);

        // 应用字体设置
        let theme = Theme::global_mut(cx);
        theme.font_family = self.font_family.clone().into();
        theme.font_size = gpui::px(self.font_size as f32);

        // 同步自动保存配置
        self.sync_auto_save_config(cx);
    }

    /// 同步自动保存配置到全局状态
    pub fn sync_auto_save_config(&self, cx: &mut App) {
        Self::update_auto_save_config(self.enable_sql_auto_save, self.sql_auto_save_interval, cx);
    }

    /// 更新自动保存配置（静态方法，避免借用冲突）
    pub fn update_auto_save_config(enabled: bool, interval_seconds: f64, cx: &mut App) {
        if let Some(config) = cx.try_global::<AutoSaveConfig>() {
            config.set_enabled(enabled);
            config.set_interval_seconds(interval_seconds);
        }
    }
}

pub fn init_settings(cx: &mut App) {
    let settings = AppSettings::load();
    // 初始化自动保存配置全局状态
    cx.set_global(AutoSaveConfig::new(
        settings.enable_sql_auto_save,
        settings.sql_auto_save_interval,
    ));
    settings.apply(cx);
    cx.set_global(settings);
}

// ============================================================================
// 快捷键设置页
// ============================================================================

/// 快捷键条目
struct ShortcutEntry {
    /// macOS 快捷键字符串（Keystroke::parse 格式）
    key_macos: &'static str,
    /// Windows/Linux 快捷键字符串（Keystroke::parse 格式）
    key_other: &'static str,
    /// 国际化翻译 key
    label_key: &'static str,
}

/// 快捷键分组
struct ShortcutGroup {
    title_key: &'static str,
    entries: &'static [ShortcutEntry],
}

const WINDOW_SHORTCUTS: &[ShortcutEntry] = &[
    ShortcutEntry {
        key_macos: "cmd-q",
        key_other: "alt-f4",
        label_key: "Settings.Shortcuts.quit_app",
    },
    ShortcutEntry {
        key_macos: "cmd-m",
        key_other: "ctrl-space",
        label_key: "Settings.Shortcuts.minimize_window",
    },
    ShortcutEntry {
        key_macos: "ctrl-cmd-f",
        key_other: "alt-enter",
        label_key: "Settings.Shortcuts.toggle_fullscreen",
    },
    ShortcutEntry {
        key_macos: "shift-escape",
        key_other: "shift-escape",
        label_key: "Settings.Shortcuts.toggle_zoom",
    },
    ShortcutEntry {
        key_macos: "ctrl-w",
        key_other: "ctrl-w",
        label_key: "Settings.Shortcuts.close_panel",
    },
];

const TAB_SHORTCUTS: &[ShortcutEntry] = &[
    ShortcutEntry {
        key_macos: "cmd-1",
        key_other: "alt-1",
        label_key: "Settings.Shortcuts.switch_tab_n",
    },
    ShortcutEntry {
        key_macos: "shift-cmd-t",
        key_other: "alt-shift-t",
        label_key: "Settings.Shortcuts.duplicate_tab",
    },
    ShortcutEntry {
        key_macos: "cmd-o",
        key_other: "alt-o",
        label_key: "Settings.Shortcuts.quick_open",
    },
    ShortcutEntry {
        key_macos: "cmd-n",
        key_other: "alt-n",
        label_key: "Settings.Shortcuts.new_connection",
    },
];

const SHORTCUT_GROUPS: &[ShortcutGroup] = &[
    ShortcutGroup {
        title_key: "Settings.Shortcuts.window",
        entries: WINDOW_SHORTCUTS,
    },
    ShortcutGroup {
        title_key: "Settings.Shortcuts.tabs",
        entries: TAB_SHORTCUTS,
    },
];

/// 渲染快捷键说明页面
pub(crate) fn render_shortcuts_section(cx: &App) -> gpui::AnyElement {
    let is_macos = cfg!(target_os = "macos");

    let mut container = v_flex().gap_4().p_4();

    for group in SHORTCUT_GROUPS {
        let mut group_container = v_flex().gap_2();

        // 分组标题
        group_container = group_container.child(
            div()
                .text_sm()
                .font_weight(FontWeight::SEMIBOLD)
                .child(t!(group.title_key).to_string()),
        );

        // 快捷键列表
        let mut list = v_flex().gap_1().pl_2();

        for entry in group.entries {
            let key_str = if is_macos {
                entry.key_macos
            } else {
                entry.key_other
            };

            let keystroke = Keystroke::parse(key_str).expect("快捷键定义非法");

            list = list.child(
                h_flex()
                    .items_center()
                    .justify_between()
                    .py_1()
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(t!(entry.label_key).to_string()),
                    )
                    .child(Kbd::new(keystroke)),
            );
        }

        group_container = group_container.child(list);
        container = container.child(group_container);
    }

    container.into_any_element()
}

/// GitHub 开源地址
const GITHUB_URL: &str = "https://github.com/feigeCode/onetcli";

/// 渲染关于页面
pub(crate) fn render_about_section(cx: &App) -> gpui::AnyElement {
    let version = env!("CARGO_PKG_VERSION");
    let muted = cx.theme().muted_foreground;

    let disclaimer_items: Vec<String> = (1..=5)
        .map(|i| {
            let key = format!("Settings.About.disclaimer_item_{}", i);
            let text = t!(&key).to_string();
            format!("{}. {}", i, text)
        })
        .collect();

    let data_safety_items: Vec<String> = (1..=3)
        .map(|i| {
            let key = format!("Settings.About.data_safety_item_{}", i);
            let text = t!(&key).to_string();
            format!("• {}", text)
        })
        .collect();

    v_flex()
        .gap_4()
        .p_4()
        // 版本信息
        .child(
            h_flex()
                .gap_2()
                .items_center()
                .child(div().text_sm().child(format!(
                    "{}: {}",
                    t!("Settings.About.version"),
                    version
                ))),
        )
        // GitHub 开源地址
        .child(
            h_flex()
                .gap_2()
                .items_center()
                .child(
                    div()
                        .text_sm()
                        .child(format!("{}: ", t!("Settings.About.opensource_label"))),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().link)
                        .child(GITHUB_URL),
                )
                .child(Clipboard::new("about-copy-github-url").value(GITHUB_URL))
                .child(
                    Button::new("about-open-github")
                        .icon(IconName::ExternalLink)
                        .xsmall()
                        .ghost()
                        .on_click(|_: &ClickEvent, _, cx| {
                            cx.open_url(GITHUB_URL);
                        }),
                ),
        )
        // 免责声明
        .child(
            v_flex()
                .gap_2()
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(t!("Settings.About.disclaimer_title").to_string()),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(muted)
                        .child(t!("Settings.About.disclaimer_status").to_string()),
                )
                .child(
                    v_flex().gap_1().pl_2().children(
                        disclaimer_items
                            .into_iter()
                            .map(|item| div().text_sm().text_color(muted).child(item)),
                    ),
                ),
        )
        // 数据与安全提示
        .child(
            v_flex()
                .gap_2()
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(t!("Settings.About.data_safety_title").to_string()),
                )
                .child(
                    v_flex().gap_1().pl_2().children(
                        data_safety_items
                            .into_iter()
                            .map(|item| div().text_sm().text_color(muted).child(item)),
                    ),
                ),
        )
        .into_any_element()
}
