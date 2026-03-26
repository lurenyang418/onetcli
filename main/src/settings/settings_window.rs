use gpui::{
    div, App, AppContext, Context, Entity, FocusHandle, Focusable, InteractiveElement, IntoElement,
    ParentElement, Render, SharedString, Styled, Window,
};
use gpui_component::setting::{
    NumberFieldOptions, SettingField, SettingGroup, SettingItem, SettingPage, Settings,
};
use gpui_component::{
    group_box::GroupBoxVariant, ActiveTheme, IconName, Sizable, Size, Theme, ThemeMode,
};
use one_core::popup_window::{open_popup_window, PopupWindowOptions};
use rust_i18n::t;

use super::setting_tab::{
    init_settings, render_about_section, render_shortcuts_section, AppSettings, DatabaseOpenMode,
};
use crate::settings::llm_providers_view::LlmProvidersView;

pub struct SettingsWindow {
    focus_handle: FocusHandle,
    size: Size,
    group_variant: GroupBoxVariant,
    llm_providers_view: Entity<LlmProvidersView>,
}

impl SettingsWindow {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let llm_providers_view = cx.new(|cx| LlmProvidersView::new(cx));
        Self {
            focus_handle: cx.focus_handle(),
            size: Size::default(),
            group_variant: GroupBoxVariant::Outline,
            llm_providers_view,
        }
    }

    fn setting_pages(&self, window: &mut Window, _cx: &App) -> Vec<SettingPage> {
        let font_names = window.text_system().all_font_names();
        let font_options: Vec<(SharedString, SharedString)> = font_names
            .into_iter()
            .map(|name| (name.clone().into(), name.into()))
            .collect();

        let llm_view = self.llm_providers_view.clone();
        let default_settings = AppSettings::default();

        vec![
            SettingPage::new(t!("Settings.General.title"))
                .resettable(true)
                .default_open(true)
                .icon(IconName::Settings)
                .groups(vec![
                    SettingGroup::new()
                        .title(t!("Settings.General.Language.group_title"))
                        .items(vec![SettingItem::new(
                            t!("Settings.General.Language.ui_language"),
                            SettingField::dropdown(
                                vec![
                                    ("zh-CN".into(), t!("Settings.General.Language.zh_cn").into()),
                                    ("zh-HK".into(), t!("Settings.General.Language.zh_hk").into()),
                                    ("en".into(), t!("Settings.General.Language.en").into()),
                                ],
                                |cx: &App| {
                                    SharedString::from(AppSettings::global(cx).locale.clone())
                                },
                                |val: SharedString, cx: &mut App| {
                                    let settings = AppSettings::global_mut(cx);
                                    settings.locale = val.to_string();
                                    gpui_component::set_locale(&settings.locale);
                                    settings.save();
                                },
                            )
                            .default_value(SharedString::from(default_settings.locale)),
                        )
                        .description(
                            t!("Settings.General.Language.ui_language_desc").to_string(),
                        )]),
                    SettingGroup::new()
                        .title(t!("Settings.General.Appearance.group_title"))
                        .items(vec![
                            SettingItem::new(
                                t!("Settings.General.Appearance.dark_mode"),
                                SettingField::switch(
                                    |cx: &App| cx.theme().mode.is_dark(),
                                    |val: bool, cx: &mut App| {
                                        let mode = if val {
                                            ThemeMode::Dark
                                        } else {
                                            ThemeMode::Light
                                        };
                                        Theme::global_mut(cx).mode = mode;
                                        Theme::change(mode, None, cx);

                                        let settings = AppSettings::global_mut(cx);
                                        settings.theme_mode = if val {
                                            "dark".to_string()
                                        } else {
                                            "light".to_string()
                                        };
                                        settings.save();
                                    },
                                )
                                .default_value(false),
                            )
                            .description(
                                t!("Settings.General.Appearance.dark_mode_desc").to_string(),
                            ),
                            SettingItem::new(
                                t!("Settings.General.Appearance.auto_switch_theme"),
                                SettingField::checkbox(
                                    |cx: &App| AppSettings::global(cx).auto_switch_theme,
                                    |val: bool, cx: &mut App| {
                                        let settings = AppSettings::global_mut(cx);
                                        settings.auto_switch_theme = val;
                                        settings.save();
                                    },
                                )
                                .default_value(default_settings.auto_switch_theme),
                            )
                            .description(
                                t!("Settings.General.Appearance.auto_switch_theme_desc")
                                    .to_string(),
                            ),
                        ]),
                    SettingGroup::new()
                        .title(t!("Settings.General.Window.group_title"))
                        .items(vec![SettingItem::new(
                            t!("Settings.General.Window.start_maximized"),
                            SettingField::switch(
                                |cx: &App| AppSettings::global(cx).start_maximized,
                                |val: bool, cx: &mut App| {
                                    let settings = AppSettings::global_mut(cx);
                                    settings.start_maximized = val;
                                    settings.save();
                                },
                            )
                            .default_value(default_settings.start_maximized),
                        )
                        .description(
                            t!("Settings.General.Window.start_maximized_desc").to_string(),
                        )]),
                    SettingGroup::new()
                        .title(t!("Settings.General.Font.group_title"))
                        .item(
                            SettingItem::new(
                                t!("Settings.General.Font.font_family"),
                                SettingField::dropdown(
                                    font_options,
                                    |cx: &App| {
                                        SharedString::from(
                                            AppSettings::global(cx).font_family.clone(),
                                        )
                                    },
                                    |val: SharedString, cx: &mut App| {
                                        let settings = AppSettings::global_mut(cx);
                                        settings.font_family = val.to_string();
                                        settings.save();
                                        gpui_component::Theme::global_mut(cx).font_family = val;
                                        if let Some(window) = cx.active_window() {
                                            let _ = cx.update_window(window, |_, window, _| {
                                                window.refresh()
                                            });
                                        }
                                    },
                                )
                                .default_value(SharedString::from(default_settings.font_family)),
                            )
                            .description(t!("Settings.General.Font.font_family_desc").to_string()),
                        )
                        .item(
                            SettingItem::new(
                                t!("Settings.General.Font.font_size"),
                                SettingField::number_input(
                                    NumberFieldOptions {
                                        min: 8.0,
                                        max: 72.0,
                                        ..Default::default()
                                    },
                                    |cx: &App| AppSettings::global(cx).font_size,
                                    |val: f64, cx: &mut App| {
                                        let settings = AppSettings::global_mut(cx);
                                        settings.font_size = val;
                                        settings.save();
                                        let theme = gpui_component::Theme::global_mut(cx);
                                        theme.font_size = gpui::px(val as f32);
                                        if let Some(window) = cx.active_window() {
                                            let _ = cx.update_window(window, |_, window, _| {
                                                window.refresh();
                                            });
                                        }
                                    },
                                )
                                .default_value(default_settings.font_size),
                            )
                            .description(t!("Settings.General.Font.font_size_desc").to_string()),
                        ),
                    SettingGroup::new()
                        .title(t!("Settings.General.Database.group_title"))
                        .items(vec![
                            SettingItem::new(
                                t!("Settings.General.Database.open_mode"),
                                SettingField::dropdown(
                                    vec![
                                        (
                                            "single".into(),
                                            t!("Settings.General.Database.single_mode").into(),
                                        ),
                                        (
                                            "workspace".into(),
                                            t!("Settings.General.Database.workspace_mode").into(),
                                        ),
                                    ],
                                    |cx: &App| {
                                        SharedString::from(
                                            AppSettings::global(cx).database_open_mode.as_str(),
                                        )
                                    },
                                    |val: SharedString, cx: &mut App| {
                                        let settings = AppSettings::global_mut(cx);
                                        settings.database_open_mode =
                                            DatabaseOpenMode::from_str(&val);
                                        settings.save();
                                    },
                                )
                                .default_value(SharedString::from(
                                    default_settings.database_open_mode.as_str(),
                                )),
                            )
                            .description(
                                t!("Settings.General.Database.open_mode_desc").to_string(),
                            ),
                            SettingItem::new(
                                t!("Settings.General.Database.auto_save"),
                                SettingField::switch(
                                    |cx: &App| AppSettings::global(cx).enable_sql_auto_save,
                                    |val: bool, cx: &mut App| {
                                        let settings = AppSettings::global_mut(cx);
                                        settings.enable_sql_auto_save = val;
                                        settings.save();
                                        AppSettings::update_auto_save_config(
                                            val,
                                            cx.global::<AppSettings>().sql_auto_save_interval,
                                            cx,
                                        );
                                    },
                                )
                                .default_value(default_settings.enable_sql_auto_save),
                            )
                            .description(
                                t!("Settings.General.Database.auto_save_desc").to_string(),
                            ),
                            SettingItem::new(
                                t!("Settings.General.Database.auto_save_interval"),
                                SettingField::number_input(
                                    NumberFieldOptions {
                                        min: 1.0,
                                        max: 60.0,
                                        step: 1.0,
                                    },
                                    |cx: &App| AppSettings::global(cx).sql_auto_save_interval,
                                    |val: f64, cx: &mut App| {
                                        let settings = AppSettings::global_mut(cx);
                                        settings.sql_auto_save_interval = val;
                                        settings.save();
                                        AppSettings::update_auto_save_config(
                                            cx.global::<AppSettings>().enable_sql_auto_save,
                                            val,
                                            cx,
                                        );
                                    },
                                )
                                .default_value(default_settings.sql_auto_save_interval),
                            )
                            .description(
                                t!("Settings.General.Database.auto_save_interval_desc").to_string(),
                            ),
                        ]),
                ]),
            // 快捷键页面
            SettingPage::new(t!("Settings.Shortcuts.title"))
                .icon(IconName::Key)
                .group(SettingGroup::new().item(SettingItem::render(
                    move |_options, _window, cx| render_shortcuts_section(cx),
                ))),
            SettingPage::new(t!("LlmProviders.title"))
                .icon(IconName::Bot)
                .group(SettingGroup::new().item(SettingItem::render(
                    move |_options, _window, _cx| llm_view.clone().into_any_element(),
                ))),
            // 关于页面
            SettingPage::new(t!("Settings.About.title"))
                .icon(IconName::Info)
                .group(SettingGroup::new().item(SettingItem::render(
                    move |_options, _window, cx| render_about_section(cx),
                ))),
        ]
    }
}

impl Focusable for SettingsWindow {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for SettingsWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !cx.has_global::<AppSettings>() {
            init_settings(cx);
        }

        div().track_focus(&self.focus_handle).size_full().child(
            Settings::new("settings-window")
                .with_size(self.size)
                .with_group_variant(self.group_variant)
                .pages(self.setting_pages(window, cx)),
        )
    }
}

pub fn open_settings_window(cx: &mut App) {
    open_popup_window(
        PopupWindowOptions::new(t!("Common.settings")).size(900.0, 650.0),
        move |_window, cx| {
            let settings_view = cx.new(|cx| SettingsWindow::new(cx));
            settings_view
        },
        cx,
    );
}
