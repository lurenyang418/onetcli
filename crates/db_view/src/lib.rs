rust_i18n::i18n!("locales", fallback = "en");

use gpui::App;

pub mod common;
pub mod connection_form_window;
pub mod database_objects_tab;
pub mod database_tab;
pub mod database_view_plugin;
mod db_tree_event;
pub mod db_tree_view;
mod import_export;
pub mod postgresql;
pub mod sql_editor;
#[cfg(test)]
mod sql_editor_completion_tests;
pub mod sql_editor_view;
pub(crate) mod sql_inline_completion;
pub mod sql_result_tab;
mod table_data;
pub mod table_data_tab;
pub mod table_designer_tab;

pub use common::DatabaseFormEvent;
pub use one_core::ai_chat::ask_ai::{AskAiButton, emit_ask_ai_event, init_ask_ai_notifier};

pub fn init(cx: &mut App) {
    table_data::data_grid::init(cx);
    database_tab::init(cx);
    sql_editor_view::init(cx);
}
