use crate::home_tab::HomePage;
use gpui::{Context, Window};
use one_core::storage::{ConnectionType, StoredConnection, Workspace};

pub(crate) trait ConnectionOpenStrategy {
    fn open(self: Box<Self>, home: &mut HomePage, window: &mut Window, cx: &mut Context<HomePage>);
}

pub(crate) fn build_connection_open_strategy(
    connection: StoredConnection,
    workspace: Option<Workspace>,
) -> Box<dyn ConnectionOpenStrategy> {
    match connection.connection_type {
        ConnectionType::Database => Box::new(DatabaseOpenStrategy {
            connection,
            workspace,
        }),
        _ => Box::new(NoopOpenStrategy),
    }
}

struct DatabaseOpenStrategy {
    connection: StoredConnection,
    workspace: Option<Workspace>,
}

impl ConnectionOpenStrategy for DatabaseOpenStrategy {
    fn open(self: Box<Self>, home: &mut HomePage, window: &mut Window, cx: &mut Context<HomePage>) {
        let DatabaseOpenStrategy {
            connection,
            workspace,
        } = *self;
        home.add_item_to_tab(&connection, workspace, window, cx);
    }
}

struct NoopOpenStrategy;

impl ConnectionOpenStrategy for NoopOpenStrategy {
    fn open(
        self: Box<Self>,
        _home: &mut HomePage,
        _window: &mut Window,
        _cx: &mut Context<HomePage>,
    ) {
    }
}