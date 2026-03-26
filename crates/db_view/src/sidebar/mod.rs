//! 数据库视图侧边栏模块
//!
//! 提供数据库视图的侧边栏功能（AI 聊天功能已移除）

use gpui::prelude::FluentBuilder;
use gpui::{
    AnyElement, App, AppContext, Context, Entity, EventEmitter, FocusHandle, Focusable,
    InteractiveElement, IntoElement, ParentElement, Render, SharedString,
    StatefulInteractiveElement, Styled, Subscription, Window, div, px,
};
use gpui_component::{ActiveTheme, Icon, IconName, Sizable, Size, v_flex};
use one_core::ai_chat::CodeBlockAction;
use one_core::layout::TOOLBAR_WIDTH;

/// 侧边栏面板类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarPanel {
    /// AI 聊天面板（已移除）
    AiChat,
}

/// 数据库侧边栏事件
#[derive(Clone, Debug)]
pub enum DatabaseSidebarEvent {
    /// 面板切换
    PanelChanged,
    /// 请求询问 AI（由外部触发，内部处理）
    AskAi,
}

/// 数据库侧边栏组件（简化版 - AI 聊天功能已移除）
pub struct DatabaseSidebar {
    /// 焦点句柄
    focus_handle: FocusHandle,
    /// 是否处于激活状态
    is_active: bool,
    /// 订阅句柄
    _subs: Vec<Subscription>,
}

impl DatabaseSidebar {
    pub fn new(
        _window: &mut Window,
        cx: &mut Context<Self>,
        _selector_context: (),
    ) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            is_active: false,
            _subs: Vec::new(),
        }
    }

    /// 设置激活状态
    pub fn set_active(&mut self, active: bool, _cx: &mut Context<Self>) {
        self.is_active = active;
    }

    /// 设置激活的面板
    pub fn set_active_panel(&mut self, _panel: Option<SidebarPanel>, _cx: &mut Context<Self>) {}

    /// 切换面板
    pub fn toggle_panel(&mut self, _panel: SidebarPanel, _cx: &mut Context<Self>) {}

    /// 是否显示侧边栏面板
    pub fn is_panel_visible(&self) -> bool {
        false
    }

    /// 询问 AI（已禁用）
    pub fn ask_ai(&mut self, _message: String, _cx: &mut Context<Self>) {
        // AI 聊天功能已移除
    }

    /// 注册代码块操作
    pub fn register_code_block_action(&self, _action: CodeBlockAction, _cx: &mut Context<Self>) {}

    /// 渲染工具栏按钮
    fn render_toolbar_button(
        &self,
        _panel: SidebarPanel,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let accent_color = cx.theme().accent;
        let accent_fg = cx.theme().accent_foreground;
        let muted_fg = cx.theme().muted_foreground;
        let muted_bg = cx.theme().muted;

        div()
            .id(SharedString::from(format!("sidebar-btn-{:?}", _panel)))
            .w(px(36.0))
            .h(px(36.0))
            .flex()
            .items_center()
            .justify_center()
            .rounded_md()
            .cursor_pointer()
            .when(false, |this| this.bg(accent_color))
            .when(true, |this| this.hover(|s| s.bg(muted_bg)))
            .child(
                Icon::new(IconName::AI)
                    .with_size(Size::Medium)
                    .text_color(muted_fg),
            )
    }

    /// 渲染工具栏
    pub fn render_toolbar(&self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let border_color = cx.theme().border;
        let muted_bg = cx.theme().muted;

        v_flex()
            .flex_shrink_0()
            .w(TOOLBAR_WIDTH)
            .h_full()
            .bg(muted_bg)
            .border_l_1()
            .border_color(border_color)
            .items_center()
            .py_2()
            .gap_1()
            .into_any_element()
    }

    /// 渲染面板内容
    pub fn render_panel_content(
        &self,
        _panel: SidebarPanel,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> AnyElement {
        div().into_any_element()
    }
}

impl EventEmitter<DatabaseSidebarEvent> for DatabaseSidebar {}

impl Focusable for DatabaseSidebar {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for DatabaseSidebar {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let border_color = cx.theme().border;

        div()
            .h_full()
            .flex_shrink_0()
            .when(!self.is_panel_visible(), |this| {
                this.child(self.render_toolbar(window, cx))
            })
    }
}
