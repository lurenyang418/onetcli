use crate::home_tab::HomePage;
use crate::setting_tab::{AppSettings, DatabaseOpenMode};
use db_view::database_tab::DatabaseTabView;
use gpui::AppContext;
use gpui::{Context, Window};
use one_core::storage::{ConnectionType, StoredConnection, Workspace};
use one_core::tab_container::TabItem;

impl HomePage {
    pub(crate) fn add_item_to_tab(
        &mut self,
        conn: &StoredConnection,
        workspace: Option<Workspace>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // 根据设置中的数据库打开方式决定如何打开
        let open_mode = if cx.has_global::<AppSettings>() {
            AppSettings::global(cx).database_open_mode
        } else {
            DatabaseOpenMode::default()
        };

        // 在 defer 之前准备所有需要的数据，避免在 HomePage 更新期间
        // 触发 on_deactivate 导致双重借用 panic
        let workspace_id = workspace.as_ref().and_then(|w| w.id);
        let conn_clone = conn.clone();
        let connections: Vec<StoredConnection> = match open_mode {
            DatabaseOpenMode::Workspace if workspace_id.is_some() => self
                .connections
                .iter()
                .filter(|c| c.workspace_id == workspace_id)
                .filter(|c| c.connection_type == ConnectionType::Database)
                .cloned()
                .collect(),
            _ => vec![conn.clone()],
        };

        let tab_container = self.tab_container.clone();
        window.defer(cx, move |window, cx| {
            tab_container.update(cx, |tc, cx| match open_mode {
                DatabaseOpenMode::Single => {
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
                }
                DatabaseOpenMode::Workspace => {
                    let tab_id = if workspace_id.is_some() {
                        format!("workspace-database-tab-{}", workspace_id.unwrap_or(0))
                    } else {
                        format!("database-tab-{}", conn_clone.id.unwrap_or(0))
                    };

                    let active_conn_id = conn_clone.id;
                    tc.activate_or_add_tab_lazy(
                        tab_id.clone(),
                        move |window, cx| {
                            let db_view = cx.new(|cx| {
                                DatabaseTabView::new_with_active_conn(
                                    workspace,
                                    connections,
                                    active_conn_id,
                                    window,
                                    cx,
                                )
                            });
                            TabItem::new(tab_id.clone(), "home", db_view)
                        },
                        window,
                        cx,
                    );
                }
            });
        });
    }

    /// 复制当前活动标签并打开
    pub(crate) fn duplicate_active_tab(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {
        // Terminal 复制功能已移除
    }

    pub(crate) fn add_api_tab(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {
        // API tab 已移除
    }
}
