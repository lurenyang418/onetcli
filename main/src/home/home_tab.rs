use db_view::connection_form_window::{ConnectionFormWindow, ConnectionFormWindowConfig};
use db_view::database_tab::DatabaseTabView;
use gpui::prelude::FluentBuilder;
use gpui::{
    AnyElement, App, AppContext, AsyncApp, Context, Entity, EventEmitter, FocusHandle,
    Focusable, FontWeight, InteractiveElement, IntoElement, KeyBinding, ParentElement, Render,
    SharedString, StatefulInteractiveElement, Styled, Subscription, Window, actions,
    div, px,
};
use gpui_component::{
    ActiveTheme, Icon, IconName, InteractiveElementExt, Sizable, Size, WindowExt,
    button::{Button, ButtonVariants as _},
    h_flex,
    input::{Input, InputState},
    list::{List, ListState},
    v_flex,
};
use one_core::connection_notifier::{ConnectionDataEvent, emit_connection_event, get_notifier};
use one_core::popup_window::{PopupWindowOptions, open_popup_window};
use one_core::storage::traits::Repository;
use one_core::storage::{
    ActiveConnections, ConnectionRepository, DatabaseType, GlobalStorageState,
    StoredConnection,
};
use one_core::tab_container::{TabContainer, TabContent, TabContentEvent, TabItem};
use rust_i18n::t;

use crate::home::home_connection_quick_open::ConnectionQuickOpenDelegate;

actions!(home_tab, [OpenConnectionQuickOpen, NewConnectionShortcut]);

pub fn init(cx: &mut App) {
    cx.bind_keys([
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-o", OpenConnectionQuickOpen, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-o", OpenConnectionQuickOpen, None),
        #[cfg(target_os = "macos")]
        KeyBinding::new("cmd-n", NewConnectionShortcut, None),
        #[cfg(not(target_os = "macos"))]
        KeyBinding::new("ctrl-n", NewConnectionShortcut, None),
    ]);
}

fn get_new_connection_label() -> String {
    #[cfg(target_os = "macos")]
    {
        format!("{} (⌘N)", t!("Home.new_connection"))
    }
    #[cfg(not(target_os = "macos"))]
    {
        format!("{} (Ctrl+N)", t!("Home.new_connection"))
    }
}

// HomePage Entity - 管理 home 页面的所有状态

pub struct HomePage {
    focus_handle: FocusHandle,
    pub(crate) connections: Vec<StoredConnection>,
    pub(crate) tab_container: Entity<TabContainer>,
    search_input: Entity<InputState>,
    pub(crate) editing_connection_id: Option<i64>,
    selected_connection_id: Option<i64>,
    pub(crate) _subscriptions: Vec<Subscription>,
    /// 复制的字段名（用于显示反馈）
    pub(crate) copied_field: Option<String>,
    /// 密码是否可见
    pub(crate) password_visible: bool,
}

impl HomePage {
    pub fn new(
        tab_container: Entity<TabContainer>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let search_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder(t!("Home.search_placeholder"))
                .clean_on_escape()
        });

        let mut page = Self {
            focus_handle: cx.focus_handle(),
            connections: Vec::new(),
            tab_container,
            search_input,
            editing_connection_id: None,
            selected_connection_id: None,
            _subscriptions: Vec::new(),
            copied_field: None,
            password_visible: false,
        };

        // 加载连接
        page.load_connections(cx);

        // 订阅全局连接事件，当连接创建/更新时刷新列表
        if let Some(notifier) = get_notifier(cx) {
            cx.subscribe(
                &notifier,
                |this, _, event: &ConnectionDataEvent, cx| match event {
                    ConnectionDataEvent::ConnectionCreated { connection } => {
                        // 立即将新连接添加到列表，避免异步加载的时序问题
                        this.connections.push(connection.clone());
                        cx.notify();
                        // 然后异步重新加载以确保数据一致性
                        this.load_connections(cx);
                        // 云同步已禁用
                    }
                    ConnectionDataEvent::ConnectionUpdated { connection } => {
                        // 立即更新列表中的连接，避免异步加载的时序问题
                        if let Some(pos) =
                            this.connections.iter().position(|c| c.id == connection.id)
                        {
                            this.connections[pos] = connection.clone();
                        } else {
                            // 如果找不到，添加到列表
                            this.connections.push(connection.clone());
                        }
                        cx.notify();
                        // 然后异步重新加载以确保数据一致性
                        this.load_connections(cx);
                        // 云同步已禁用
                    }
                    ConnectionDataEvent::ConnectionDeleted { connection_id } => {
                        // 立即从列表中移除连接
                        this.connections.retain(|c| c.id != Some(*connection_id));
                        cx.notify();
                        // 然后异步重新加载以确保数据一致性
                        this.load_connections(cx);
                        // 云同步已禁用
                    }
                    ConnectionDataEvent::WorkspaceCreated { .. }
                    | ConnectionDataEvent::WorkspaceUpdated { .. }
                    | ConnectionDataEvent::WorkspaceDeleted { .. } => {
                        // Workspace 事件已禁用，忽略
                    }
                    ConnectionDataEvent::SchemaChanged { .. } => {
                        // SchemaChanged 由 db_tree_view 处理，此处无需操作
                    }
                },
            )
            .detach();
        }

        page
    }

    fn load_connections(&mut self, cx: &mut Context<Self>) {
        let storage = cx.global::<GlobalStorageState>().storage.clone();
        cx.spawn(async move |this, cx: &mut AsyncApp| {
            let result = (|| {
                let repo = storage
                    .get::<ConnectionRepository>()
                    .ok_or_else(|| anyhow::anyhow!("ConnectionRepository not found"))?;
                repo.list()
            })();

            match result {
                Ok(connections) => {
                    _ = this.update(cx, |this, cx| {
                        this.connections = connections;
                        cx.notify();
                    });
                }
                Err(e) => {
                    tracing::error!("Task join error: {}", e);
                }
            }
        })
        .detach();
    }

    fn refresh_local_home_data(&mut self, cx: &mut Context<Self>) {
        self.load_connections(cx);
    }

    fn confirm_edit_connection(
        &mut self,
        conn_id: i64,
        conn_name: String,
        db_type: Option<DatabaseType>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let is_active = cx.global::<ActiveConnections>().is_active(conn_id);

        if is_active {
            window.open_dialog(cx, move |dialog, _window, _cx| {
                dialog
                    .title(t!("Connection.in_use_title").to_string().into_any_element())
                    .child(
                        t!("Connection.in_use_cannot_edit", conn_name = conn_name)
                            .to_string()
                            .into_any_element(),
                    )
                    .alert()
            });
        } else if let Some(db_type) = db_type {
            self.editing_connection_id = Some(conn_id);
            self.show_connection_form(db_type, window, cx);
        }
    }

    fn confirm_delete_connection(
        &mut self,
        conn_id: i64,
        conn_name: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let is_active = cx.global::<ActiveConnections>().is_active(conn_id);
        let view = cx.entity().clone();

        if is_active {
            window.open_dialog(cx, move |dialog, _window, _cx| {
                dialog
                    .title(t!("Connection.in_use_title").to_string().into_any_element())
                    .child(
                        t!("Connection.in_use_cannot_delete", conn_name = conn_name)
                            .to_string()
                            .into_any_element(),
                    )
                    .alert()
            });
        } else {
            window.open_dialog(cx, move |dialog, _window, _cx| {
                let view_clone = view.clone();
                dialog
                    .title(t!("Common.delete").to_string().into_any_element())
                    .child(
                        t!("Connection.delete_confirm", conn_name = conn_name)
                            .to_string()
                            .into_any_element(),
                    )
                    .confirm()
                    .on_ok(move |_, _, cx| {
                        let _ = view_clone.update(cx, |this, cx| {
                            this.delete_connection(conn_id, cx);
                        });
                        true
                    })
            });
        }
    }

    fn delete_connection(&mut self, conn_id: i64, cx: &mut Context<Self>) {
        let storage = cx.global::<GlobalStorageState>().storage.clone();

        cx.spawn(async move |this, cx: &mut AsyncApp| {
            // 删除本地连接
            let result = (|| {
                let repo = storage
                    .get::<ConnectionRepository>()
                    .ok_or_else(|| anyhow::anyhow!("ConnectionRepository not found"))?;
                repo.delete(conn_id)
            })();

            match result {
                Ok(_) => {
                    _ = this.update(cx, |this, cx| {
                        this.connections.retain(|c| c.id != Some(conn_id));
                        if this.selected_connection_id == Some(conn_id) {
                            this.selected_connection_id = None;
                        }
                        emit_connection_event(
                            ConnectionDataEvent::ConnectionDeleted {
                                connection_id: conn_id,
                            },
                            cx,
                        );
                        cx.notify();
                    });
                }
                Err(e) => {
                    tracing::error!("Failed to delete connection: {}", e);
                }
            }
        })
        .detach();
    }

    pub(crate) fn show_connection_quick_open(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let parent = cx.entity();
        let connections = self.connections.clone();
        let list = cx.new(|cx| {
            let mut delegate = ConnectionQuickOpenDelegate::new(parent);
            delegate.update_items(&connections);
            ListState::new(delegate, window, cx).searchable(true)
        });

        let list_for_focus = list.clone();
        window.open_dialog(cx, move |dialog, _window, cx| {
            dialog
                .title("打开连接".to_string())
                .w(px(520.0))
                .child(
                    v_flex().gap_2().child(
                        List::new(&list)
                            .w_full()
                            .max_h(px(360.0))
                            .p(px(8.0))
                            .border_1()
                            .border_color(cx.theme().border)
                            .rounded(cx.theme().radius),
                    ),
                )
                .alert()
                .button_props(
                    gpui_component::dialog::DialogButtonProps::default()
                        .ok_text(t!("Common.close")),
                )
        });
        // 将焦点设置到 List 搜索框，使上下键和 Enter 键可用
        list_for_focus.update(cx, |state, cx| {
            state.focus(window, cx);
        });
    }

    pub(crate) fn show_new_connection_dialog(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // 直接打开 PostgreSQL 连接表单，无需选择
        self.editing_connection_id = None;
        self.show_connection_form(DatabaseType::PostgreSQL, window, cx);
    }

    pub(crate) fn open_connection_from_quick(
        &mut self,
        connection: &StoredConnection,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.add_item_to_tab(connection, window, cx);
        cx.notify();
    }

    pub(crate) fn show_connection_form(
        &mut self,
        db_type: DatabaseType,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let editing_conn = self
            .editing_connection_id
            .and_then(|id| self.connections.iter().find(|c| c.id == Some(id)).cloned());

        let config = ConnectionFormWindowConfig {
            db_type,
            editing_connection: editing_conn,
        };

        self.editing_connection_id = None;

        open_popup_window(
            PopupWindowOptions::new(if config.editing_connection.is_some() {
                t!("Connection.edit", db_type = db_type.as_str()).to_string()
            } else {
                t!("Connection.new", db_type = db_type.as_str()).to_string()
            })
            .size(720.0, 780.0),
            move |window, cx| cx.new(|cx| ConnectionFormWindow::new(config, window, cx)),
            cx,
        );
    }

    fn render_toolbar(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .gap_3()
            .px_4()
            .py_2()
            .border_b_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().background)
            .items_center()
            // ===== 中间弹性空间 =====
            .child(div().flex_1())
            // ===== 右侧操作区 =====
            .child(
                h_flex()
                    .gap_1()
                    .items_center()
                    // 搜索框
                    .child(
                        Input::new(&self.search_input)
                            .cleanable(true)
                            .w(px(240.0))
                            .bg(cx.theme().muted),
                    )
                    // 刷新按钮
                    .child(
                        Button::new("refresh-button")
                            .icon(IconName::Refresh)
                            .ghost()
                            .tooltip(t!("Home.refresh"))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.refresh_local_home_data(cx);
                            })),
                    )
            )
    }

    /// 复制到剪贴板并显示反馈
    fn copy_to_clipboard(&mut self, label: String, value: String, cx: &mut Context<Self>) {
        cx.write_to_clipboard(gpui::ClipboardItem::new_string(value));
        self.copied_field = Some(label);
        cx.notify();
    }

    /// 切换密码可见性
    fn toggle_password_visibility(&mut self, cx: &mut Context<Self>) {
        self.password_visible = !self.password_visible;
        cx.notify();
    }

    fn render_sidebar(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // 简化的连接列表（不按工作区分组）
        let connections: Vec<_> = self.connections.clone();

        v_flex()
            .w(px(220.0))
            .h_full()
            .bg(cx.theme().sidebar)
            .border_r_1()
            .border_color(cx.theme().border)
            .child(
                div()
                    .flex_1()
                    .overflow_hidden()
                    .child(
                        v_flex()
                            .flex_1()
                            .w_full()
                            .p_2()
                            .gap_1()
                            .children(connections.iter().map(|conn| {
                                self.render_sidebar_connection_item(conn.clone(), self.selected_connection_id, cx)
                            })),
                    ),
            )
            .child(
                // 底部区域：设置
                v_flex()
                    .w_full()
                    .p_4()
                    .gap_3()
                    .border_t_1()
                    .border_color(cx.theme().border)
                    .child(
                        Button::new("open_settings")
                            .icon(IconName::Settings)
                            .label(t!("Common.settings"))
                            .w_full()
                            .justify_start()
                            .on_click(move |_, _window, cx| {
                                crate::settings::settings_window::open_settings_window(cx);
                            }),
                    )
            )
    }

    fn render_content_area(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        // 如果没有任何连接，显示空状态
        if self.connections.is_empty() {
            return self.render_empty_state(window, cx).into_any_element();
        }

        // 如果有选中的连接，显示详情
        if let Some(selected_id) = self.selected_connection_id {
            if let Some(conn) = self.connections.iter().find(|c| c.id == Some(selected_id)) {
                return self.render_connection_detail(conn.clone(), window, cx).into_any_element();
            }
        }

        // 否则显示欢迎信息
        self.render_welcome(cx).into_any_element()
    }

    fn render_empty_state(&self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity().clone();
        v_flex()
            .size_full()
            .items_center()
            .justify_center()
            .gap_4()
            .child(
                Icon::new(IconName::Database)
                    .color()
                    .with_size(Size::Large),
            )
            .child(
                div()
                    .text_lg()
                    .text_color(cx.theme().foreground)
                    .child(t!("Home.no_connections")),
            )
            .child(
                Button::new("create_first_connection")
                    .icon(IconName::Plus)
                    .label(get_new_connection_label())
                    .bg(cx.theme().primary)
                    .text_color(cx.theme().primary_foreground)
                    .on_click(move |_, window, cx| {
                        view.update(cx, |this, cx| {
                            this.show_connection_form(DatabaseType::PostgreSQL, window, cx);
                        });
                    }),
            )
    }

    fn render_welcome(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .items_center()
            .justify_center()
            .gap_4()
            .child(
                Icon::new(IconName::Database)
                    .color()
                    .with_size(Size::Large),
            )
            .child(
                div()
                    .text_lg()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(cx.theme().foreground)
                    .child(t!("Home.welcome")),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(t!("Home.welcome_hint")),
            )
    }

    fn render_connection_detail(
        &self,
        conn: StoredConnection,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        if let Ok(params) = conn.to_db_connection() {
            let dsn_display = format!(
                "{}://{}:****@{}:{}{}",
                "pgsql",
                params.username,
                params.host,
                params.port,
                params.database.as_ref().map(|d| format!("/{}", d)).unwrap_or_default()
            );
            let dsn_real = format!(
                "{}://{}:{}@{}:{}{}",
                "pgsql",
                params.username,
                params.password,
                params.host,
                params.port,
                params.database.as_ref().map(|d| format!("/{}", d)).unwrap_or_default()
            );

            v_flex()
                .size_full()
                .items_center()
                .justify_start()
                .p_8()
                .gap_6()
                .child(
                    div()
                        .text_xl()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(cx.theme().foreground)
                        .child(conn.name.clone()),
                )
                .child(
                    v_flex()
                        .w(px(700.0))
                        .gap_4()
                        .p_6()
                        .rounded_lg()
                        .border_1()
                        .border_color(cx.theme().border)
                        .bg(cx.theme().background)
                        .shadow_sm()
                        .child(self.render_detail_field(
                            t!("ConnectionForm.labels.host").to_string(),
                            params.host.clone(),
                            cx,
                        ))
                        .child(self.render_detail_field(
                            t!("ConnectionForm.labels.port").to_string(),
                            params.port.to_string(),
                            cx,
                        ))
                        .child(self.render_detail_field(
                            t!("ConnectionForm.labels.database").to_string(),
                            params.database.clone().unwrap_or_default(),
                            cx,
                        ))
                        .child(self.render_detail_field(
                            t!("ConnectionForm.labels.username").to_string(),
                            params.username.clone(),
                            cx,
                        ))
                        .child(
                            h_flex()
                                .w_full()
                                .items_center()
                                .gap_4()
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(cx.theme().muted_foreground)
                                        .w(px(100.0))
                                        .child(t!("ConnectionForm.labels.password")),
                                )
                                .child(
                                    h_flex()
                                        .flex_1()
                                        .items_center()
                                        .gap_2()
                                        .child(
                                            div()
                                                .text_sm()
                                                .text_color(cx.theme().foreground)
                                                .child(
                                                    if self.password_visible {
                                                        params.password.clone()
                                                    } else {
                                                        "********".to_string()
                                                    },
                                                ),
                                        ),
                                )
                                .child(
                                    h_flex()
                                        .items_center()
                                        .gap_1p5()
                                        .when(self.password_visible, |this| {
                                            this.child(
                                                Button::new("toggle-password-visibility")
                                                    .icon(IconName::EyeOff)
                                                    .with_size(Size::Small)
                                                    .ghost()
                                                    .on_click(cx.listener(|this, _, _, cx| {
                                                        this.toggle_password_visibility(cx);
                                                    })),
                                            )
                                        })
                                        .when(!self.password_visible, |this| {
                                            this.child(
                                                Button::new("toggle-password-visibility")
                                                    .icon(IconName::Eye)
                                                    .with_size(Size::Small)
                                                    .ghost()
                                                    .on_click(cx.listener(|this, _, _, cx| {
                                                        this.toggle_password_visibility(cx);
                                                    })),
                                            )
                                        })
                                        .when(self.copied_field == Some("password".to_string()), |this| {
                                            this.child(
                                                Button::new("copy-password")
                                                    .icon(IconName::CircleCheck)
                                                    .with_size(Size::Small)
                                                    .ghost()
                                                    .text_color(cx.theme().success),
                                            )
                                        })
                                        .when(self.copied_field != Some("password".to_string()), |this| {
                                            this.child(
                                                Button::new("copy-password")
                                                    .icon(IconName::Copy)
                                                    .with_size(Size::Small)
                                                    .ghost()
                                                    .on_click(cx.listener(move |this, _, _, cx| {
                                                        this.copy_to_clipboard("password".to_string(), params.password.clone(), cx);
                                                    })),
                                            )
                                        }),
                                ),
                        )
                        .child(self.render_detail_field_with_copy_value(
                            t!("ConnectionForm.labels.dsn").to_string(),
                            dsn_display.clone(),
                            dsn_real,
                            cx,
                        ))
                )
        } else {
            v_flex()
                .size_full()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(t!("Home.connection_invalid")),
                )
        }
    }

    fn render_detail_field(&self, label: String, value: String, cx: &mut Context<Self>) -> impl IntoElement {
        self.render_detail_field_with_copy_value(label, value.clone(), value, cx)
    }

    fn render_detail_field_with_copy_value(&self, label: String, display_value: String, copy_value: String, cx: &mut Context<Self>) -> impl IntoElement {
        let copy_value_clone = copy_value.clone();
        let label_for_check = label.clone();
        let label_for_copy = label.clone();
        let is_copied = self.copied_field == Some(label.clone());

        h_flex()
            .w_full()
            .items_center()
            .justify_between()
            .gap_4()
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .w(px(100.0))
                    .child(label.clone()),
            )
            .child(
                h_flex()
                    .flex_1()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().foreground)
                            .flex_1()
                            .overflow_hidden()
                            .text_ellipsis()
                            .whitespace_nowrap()
                            .child(display_value),
                    )
                    .when(!is_copied, |this| {
                        this.child(
                            Button::new(format!("copy-{}", label_for_copy))
                                .icon(IconName::Copy)
                                .with_size(Size::Small)
                                .ghost()
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.copy_to_clipboard(label_for_copy.clone(), copy_value_clone.clone(), cx);
                                })),
                        )
                    })
                    .when(is_copied, |this| {
                        this.child(
                            Button::new(format!("copy-{}", label_for_check))
                                .icon(IconName::CircleCheck)
                                .with_size(Size::Small)
                                .ghost()
                                .text_color(cx.theme().success),
                        )
                    }),
            )
    }

    fn render_sidebar_connection_item(
        &self,
        conn: StoredConnection,
        selected_id: Option<i64>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let conn_name = conn.name.clone();
        let conn_id = conn.id;
        let is_selected = selected_id == conn.id;
        let theme = cx.theme().clone();
        let clone_conn = conn.clone();
        let edit_conn = conn.clone();
        let delete_conn_id = conn.id;
        let delete_conn_name = conn.name.clone();

        let item = div()
            .id(SharedString::from(format!("sidebar-conn-{}", conn_id.unwrap_or(0))))
            .w_full()
            .px_2()
            .py_1p5()
            .rounded_md()
            .relative()
            .cursor_pointer()
            .bg(if is_selected { theme.list_active } else { theme.sidebar })
            .group("")
            .on_click(cx.listener(move |this, _, _, cx| {
                this.selected_connection_id = conn_id;
                cx.notify();
            }))
            .on_double_click(cx.listener(move |this, _, w, cx| {
                this.add_item_to_tab(&clone_conn, w, cx);
            }))
            .child(
                h_flex()
                    .items_center()
                    .gap_2()
                    .child(
                        Icon::new(IconName::Database)
                            .color()
                            .with_size(Size::Small),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.foreground)
                            .text_ellipsis()
                            .overflow_hidden()
                            .child(conn_name.clone()),
                    ),
            )
            .child(
                // hover时显示的编辑和删除按钮
                h_flex()
                    .absolute()
                    .top_1()
                    .right_1()
                    .gap_1()
                    .group_hover("", |style| style.opacity(1.0))
                    .opacity(0.0)
                    .child(
                        Button::new(SharedString::from(format!("edit-{}", conn_id.unwrap_or(0))))
                            .icon(IconName::Edit)
                            .with_size(Size::Small)
                            .ghost()
                            .tooltip(t!("Home.edit_connection"))
                            .on_click(cx.listener(
                                move |this, _, window, cx| {
                                    cx.stop_propagation();
                                    if let Some(conn_id) = edit_conn.id {
                                        let conn_name = edit_conn.name.clone();
                                        if let Ok(params) = edit_conn.to_db_connection() {
                                            this.confirm_edit_connection(
                                                conn_id,
                                                conn_name,
                                                Some(params.database_type),
                                                window,
                                                cx,
                                            );
                                        }
                                    }
                                },
                            )),
                    )
                    .child(
                        Button::new(SharedString::from(format!("delete-{}", conn_id.unwrap_or(0))))
                            .icon(IconName::Remove)
                            .with_size(Size::Small)
                            .ghost()
                            .tooltip(t!("Home.delete_connection"))
                            .on_click(cx.listener(
                                move |this, _, window, cx| {
                                    cx.stop_propagation();
                                    if let Some(conn_id) = delete_conn_id {
                                        let conn_name = delete_conn_name.clone();
                                        this.confirm_delete_connection(
                                            conn_id,
                                            conn_name,
                                            window,
                                            cx,
                                        );
                                    }
                                },
                            )),
                    ),
            );

        item.into_any_element()
    }

    pub(crate) fn add_item_to_tab(
        &mut self,
        conn: &StoredConnection,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let conn_clone = conn.clone();
        let tab_container = self.tab_container.clone();

        window.defer(cx, move |window, cx| {
            tab_container.update(cx, |tc, cx| {
                let tab_id = format!("database-tab-{}", conn_clone.id.unwrap_or(0));
                tc.activate_or_add_tab_lazy(
                    tab_id.clone(),
                    move |window, cx| {
                        let db_view = cx.new(|cx| {
                            DatabaseTabView::new_with_active_conn(
                                None,
                                vec![conn_clone.clone()],
                                conn_clone.id,
                                window,
                                cx,
                            )
                        });
                        TabItem::new(tab_id.clone(), "home", db_view)
                    },
                    window,
                    cx,
                );
            });
        });
    }

}

impl Focusable for HomePage {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EventEmitter<TabContentEvent> for HomePage {}

impl TabContent for HomePage {
    fn content_key(&self) -> &'static str {
        "Home"
    }

    fn title(&self, _cx: &App) -> SharedString {
        SharedString::from(t!("Home.title"))
    }

    fn icon(&self, _cx: &App) -> Option<Icon> {
        Some(IconName::Home.color())
    }

    fn closeable(&self, _cx: &App) -> bool {
        false
    }

    fn width_size(&self, _cx: &App) -> Option<Size> {
        Some(Size::Small)
    }

    fn on_activate(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        // 每次激活首页 tab 时，清除选中状态，显示欢迎页
        self.selected_connection_id = None;
        cx.notify();
    }
}

impl Render for HomePage {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div().size_full().track_focus(&self.focus_handle).child(
            h_flex()
                .size_full()
                .child(self.render_sidebar(window, cx))
                .child(
                    v_flex()
                        .flex_1()
                        .h_full()
                        .bg(cx.theme().background)
                        .child(self.render_toolbar(window, cx))
                        .child(
                            div()
                                .flex_1()
                                .w_full()
                                .overflow_hidden()
                                .bg(cx.theme().muted)
                                .child(self.render_content_area(window, cx)),
                        ),
                ),
        )
    }
}
