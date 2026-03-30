use db::{ColumnInfo, FieldType, FilterOperator};
use gpui::{
    prelude::*, px, App, AppContext, ClickEvent, Context, Entity, EventEmitter, IntoElement,
    Render, SharedString, Styled, Subscription, Window,
};
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::input::{Input, InputEvent, InputState};
use gpui_component::select::{SearchableVec, Select, SelectEvent, SelectItem, SelectState};
use gpui_component::{ActiveTheme, Disableable, IconName, Sizable};

#[derive(Clone)]
pub struct TableSchema {
    pub columns: Vec<ColumnInfo>,
}

// ============================================================================
// 字段选择项（用于 Select 组件）
// ============================================================================

#[derive(Clone, Debug)]
pub struct ColumnSelectItem {
    pub column: ColumnInfo,
}

impl SelectItem for ColumnSelectItem {
    type Value = String;

    fn title(&self) -> SharedString {
        self.column.name.clone().into()
    }

    fn value(&self) -> &Self::Value {
        &self.column.name
    }

    fn matches(&self, query: &str) -> bool {
        self.column
            .name
            .to_lowercase()
            .contains(&query.to_lowercase())
    }
}

// ============================================================================
// 操作符选择项（用于 Select 组件）
// ============================================================================

#[derive(Clone, Debug)]
pub struct OperatorSelectItem {
    pub operator: FilterOperator,
    pub label: &'static str,
}

impl SelectItem for OperatorSelectItem {
    type Value = FilterOperator;

    fn title(&self) -> SharedString {
        self.label.into()
    }

    fn value(&self) -> &Self::Value {
        &self.operator
    }

    fn matches(&self, query: &str) -> bool {
        self.label.to_lowercase().contains(&query.to_lowercase())
    }
}

// ============================================================================
// 值输入模式
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueInputMode {
    SingleValue, // =, !=, >, <, >=, <=, LIKE
    DualValue,   // BETWEEN（两个输入框）
    ListValue,   // IN, NOT IN（逗号分隔）
    NoValue,     // IS NULL, IS NOT NULL
}

impl ValueInputMode {
    pub fn from_operator(op: FilterOperator) -> Self {
        match op {
            FilterOperator::In | FilterOperator::NotIn => ValueInputMode::ListValue,
            FilterOperator::IsNull | FilterOperator::IsNotNull => ValueInputMode::NoValue,
            FilterOperator::Between => ValueInputMode::DualValue,
            FilterOperator::IsTrue | FilterOperator::IsFalse => ValueInputMode::NoValue,
            _ => ValueInputMode::SingleValue,
        }
    }
}

// ============================================================================
// 根据字段类型获取可用操作符
// ============================================================================

fn get_operators_for_field_type(field_type: FieldType) -> Vec<OperatorSelectItem> {
    match field_type {
        FieldType::Text | FieldType::LongText => vec![
            OperatorSelectItem {
                operator: FilterOperator::Like,
                label: "LIKE",
            },
            OperatorSelectItem {
                operator: FilterOperator::NotLike,
                label: "NOT LIKE",
            },
            OperatorSelectItem {
                operator: FilterOperator::Equal,
                label: "=",
            },
            OperatorSelectItem {
                operator: FilterOperator::NotEqual,
                label: "!=",
            },
            OperatorSelectItem {
                operator: FilterOperator::In,
                label: "IN",
            },
            OperatorSelectItem {
                operator: FilterOperator::NotIn,
                label: "NOT IN",
            },
            OperatorSelectItem {
                operator: FilterOperator::IsNull,
                label: "IS NULL",
            },
            OperatorSelectItem {
                operator: FilterOperator::IsNotNull,
                label: "IS NOT NULL",
            },
        ],
        FieldType::Integer | FieldType::Decimal => vec![
            OperatorSelectItem {
                operator: FilterOperator::Equal,
                label: "=",
            },
            OperatorSelectItem {
                operator: FilterOperator::NotEqual,
                label: "!=",
            },
            OperatorSelectItem {
                operator: FilterOperator::GreaterThan,
                label: ">",
            },
            OperatorSelectItem {
                operator: FilterOperator::LessThan,
                label: "<",
            },
            OperatorSelectItem {
                operator: FilterOperator::GreaterOrEqual,
                label: ">=",
            },
            OperatorSelectItem {
                operator: FilterOperator::LessOrEqual,
                label: "<=",
            },
            OperatorSelectItem {
                operator: FilterOperator::Between,
                label: "BETWEEN",
            },
            OperatorSelectItem {
                operator: FilterOperator::In,
                label: "IN",
            },
            OperatorSelectItem {
                operator: FilterOperator::NotIn,
                label: "NOT IN",
            },
            OperatorSelectItem {
                operator: FilterOperator::IsNull,
                label: "IS NULL",
            },
            OperatorSelectItem {
                operator: FilterOperator::IsNotNull,
                label: "IS NOT NULL",
            },
        ],
        FieldType::Date | FieldType::Time | FieldType::DateTime => vec![
            OperatorSelectItem {
                operator: FilterOperator::Equal,
                label: "=",
            },
            OperatorSelectItem {
                operator: FilterOperator::NotEqual,
                label: "!=",
            },
            OperatorSelectItem {
                operator: FilterOperator::GreaterThan,
                label: ">",
            },
            OperatorSelectItem {
                operator: FilterOperator::LessThan,
                label: "<",
            },
            OperatorSelectItem {
                operator: FilterOperator::GreaterOrEqual,
                label: ">=",
            },
            OperatorSelectItem {
                operator: FilterOperator::LessOrEqual,
                label: "<=",
            },
            OperatorSelectItem {
                operator: FilterOperator::Between,
                label: "BETWEEN",
            },
            OperatorSelectItem {
                operator: FilterOperator::IsNull,
                label: "IS NULL",
            },
            OperatorSelectItem {
                operator: FilterOperator::IsNotNull,
                label: "IS NOT NULL",
            },
        ],
        FieldType::Boolean => vec![
            OperatorSelectItem {
                operator: FilterOperator::IsTrue,
                label: "TRUE",
            },
            OperatorSelectItem {
                operator: FilterOperator::IsFalse,
                label: "FALSE",
            },
            OperatorSelectItem {
                operator: FilterOperator::IsNull,
                label: "IS NULL",
            },
            OperatorSelectItem {
                operator: FilterOperator::IsNotNull,
                label: "IS NOT NULL",
            },
        ],
        _ => vec![
            OperatorSelectItem {
                operator: FilterOperator::Equal,
                label: "=",
            },
            OperatorSelectItem {
                operator: FilterOperator::NotEqual,
                label: "!=",
            },
            OperatorSelectItem {
                operator: FilterOperator::IsNull,
                label: "IS NULL",
            },
            OperatorSelectItem {
                operator: FilterOperator::IsNotNull,
                label: "IS NOT NULL",
            },
        ],
    }
}

// ============================================================================
// 条件行状态
// ============================================================================

struct FilterRowState {
    row_id: u64,
    column_select: Entity<SelectState<SearchableVec<ColumnSelectItem>>>,
    operator_select: Entity<SelectState<SearchableVec<OperatorSelectItem>>>,
    value_input: Entity<InputState>,
    value2_input: Entity<InputState>,
    current_input_mode: ValueInputMode,
    subscriptions: Vec<Subscription>,
}

impl FilterRowState {
    fn new(
        row_id: u64,
        schema: &TableSchema,
        window: &mut Window,
        cx: &mut Context<TableFilterEditor>,
    ) -> Self {
        let column_items: Vec<ColumnSelectItem> = schema
            .columns
            .iter()
            .map(|col| ColumnSelectItem {
                column: col.clone(),
            })
            .collect();

        let column_select = cx.new(|cx| {
            SelectState::new(SearchableVec::new(column_items), None, window, cx).searchable(true)
        });

        let operator_select =
            cx.new(|cx| SelectState::new(SearchableVec::new(Vec::new()), None, window, cx));

        let value_input = cx.new(|cx| InputState::new(window, cx));

        let value2_input = cx.new(|cx| InputState::new(window, cx));

        Self {
            row_id,
            column_select,
            operator_select,
            value_input,
            value2_input,
            current_input_mode: ValueInputMode::SingleValue,
            subscriptions: Vec::new(),
        }
    }

    /// 清除值输入框
    fn clear_value(&self, window: &mut Window, cx: &mut Context<TableFilterEditor>) {
        self.value_input.update(cx, |state, cx| {
            state.set_value("".to_string(), window, cx);
        });
        self.value2_input.update(cx, |state, cx| {
            state.set_value("".to_string(), window, cx);
        });
    }

    /// 设置指定列的操作符列表
    fn update_operators_for_column(
        &self,
        column: &ColumnInfo,
        window: &mut Window,
        cx: &mut Context<TableFilterEditor>,
    ) {
        use gpui_component::IndexPath;

        let field_type = FieldType::from_db_type(&column.data_type);
        let operators = get_operators_for_field_type(field_type);
        self.operator_select.update(cx, |state, cx| {
            state.set_items(SearchableVec::new(operators), window, cx);
            // 重置选择为第一个可用操作符
            state.set_selected_index(Some(IndexPath::new(0)), window, cx);
        });
    }
}

// ============================================================================
// 事件定义
// ============================================================================

pub enum FilterEditorEvent {
    QueryApply,
}

// ============================================================================
// 可视化 WHERE 条件构建器
// ============================================================================

pub struct TableFilterEditor {
    condition_rows: Vec<FilterRowState>,
    schema: TableSchema,
    next_row_id: u64,
    needs_init: bool,
    _subs: Vec<Subscription>,
}

impl TableFilterEditor {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            condition_rows: Vec::new(),
            schema: TableSchema {
                columns: Vec::new(),
            },
            next_row_id: 1,
            needs_init: false,
            _subs: Vec::new(),
        }
    }

    /// 添加新条件行
    fn add_condition_row(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let row_id = self.next_row_id;
        self.next_row_id += 1;

        let row = FilterRowState::new(row_id, &self.schema, window, cx);
        self.condition_rows.push(row);
        self.setup_row_subscriptions(row_id, window, cx);
    }

    /// 删除条件行（至少保留一行），删除后触发查询
    fn remove_condition_row(&mut self, row_id: u64, cx: &mut Context<Self>) {
        if self.condition_rows.len() > 1 {
            self.condition_rows.retain(|r| r.row_id != row_id);
            cx.emit(FilterEditorEvent::QueryApply);
        }
    }

    /// 设置行的事件订阅
    fn setup_row_subscriptions(
        &mut self,
        row_id: u64,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // 找到这个行
        let row = self.condition_rows.iter_mut().find(|r| r.row_id == row_id);
        let Some(row) = row else { return };

        // 订阅字段选择事件
        let row_id_for_column = row_id;
        let schema = self.schema.clone();
        let sub = cx.subscribe_in(
            &row.column_select,
            window,
            move |this, _, event, window, cx| {
                if let SelectEvent::Confirm(Some(column_name)) = event {
                    let row = this.condition_rows.iter_mut().find(|r| r.row_id == row_id_for_column);
                    let Some(row) = row else { return };
                    let column = schema.columns.iter().find(|c| c.name == *column_name);
                    let Some(column) = column else { return };
                    let field_type = FieldType::from_db_type(&column.data_type);
                    let operators = get_operators_for_field_type(field_type);
                    // 先获取第一个操作符的 input_mode
                    let first_input_mode = operators.first().map(|op| ValueInputMode::from_operator(op.operator));
                    row.operator_select.update(cx, |state, cx| {
                        state.set_items(SearchableVec::new(operators), window, cx);
                        state.set_selected_index(Some(gpui_component::IndexPath::new(0)), window, cx);
                    });
                    // 根据新字段类型更新 input_mode
                    if let Some(input_mode) = first_input_mode {
                        row.current_input_mode = input_mode;
                    }
                    row.clear_value(window, cx);
                    // 如果第一个操作符无需输入值（TRUE/FALSE/IS NULL/IS NOT NULL），自动触发查询
                    if first_input_mode == Some(ValueInputMode::NoValue) {
                        cx.emit(FilterEditorEvent::QueryApply);
                    }
                    cx.notify();
                }
            },
        );
        row.subscriptions.push(sub);

        // 订阅操作符选择事件
        let row_id_for_operator = row_id;
        let sub = cx.subscribe_in(
            &row.operator_select,
            window,
            move |this, _, event, window, cx| {
                if let SelectEvent::Confirm(Some(operator)) = event {
                    let row = this.condition_rows.iter_mut().find(|r| r.row_id == row_id_for_operator);
                    let Some(row) = row else { return };
                    let input_mode = ValueInputMode::from_operator(*operator);
                    row.current_input_mode = input_mode;
                    // 清空值输入框
                    row.value_input.update(cx, |state, cx| {
                        state.set_value("".to_string(), window, cx);
                    });
                    row.value2_input.update(cx, |state, cx| {
                        state.set_value("".to_string(), window, cx);
                    });
                    // 无需输入值的操作符（TRUE/FALSE/IS NULL/IS NOT NULL），选择后直接触发查询
                    if input_mode == ValueInputMode::NoValue {
                        cx.emit(FilterEditorEvent::QueryApply);
                    }
                    cx.notify();
                }
            },
        );
        row.subscriptions.push(sub);

        // 订阅值输入框回车事件
        let sub = cx.subscribe_in(
            &row.value_input,
            window,
            |_, _, event: &InputEvent, _window, cx| {
                if let InputEvent::PressEnter { .. } = event {
                    cx.emit(FilterEditorEvent::QueryApply);
                }
            },
        );
        row.subscriptions.push(sub);
    }

    pub fn get_where_clause(&self, cx: &App) -> String {
        let conditions: Vec<String> = self
            .condition_rows
            .iter()
            .filter_map(|row| {
                let column = row.column_select.read(cx).selected_value()?.clone();
                let operator = row.operator_select.read(cx).selected_value()?;
                let value = row.value_input.read(cx).text().to_string();
                let value2 = row.value2_input.read(cx).text().to_string();

                let input_mode = ValueInputMode::from_operator(*operator);
                let sql_op = operator.to_sql();

                match input_mode {
                    ValueInputMode::NoValue => Some(format!("{} {}", column, sql_op)),
                    ValueInputMode::SingleValue => {
                        if value.is_empty() {
                            return None;
                        }
                        let formatted = Self::format_sql_value(&value, *operator);
                        Some(format!("{} {} {}", column, sql_op, formatted))
                    }
                    ValueInputMode::DualValue => {
                        if value.is_empty() || value2.is_empty() {
                            return None;
                        }
                        let v1 = Self::format_sql_value(&value, *operator);
                        let v2 = Self::format_sql_value(&value2, *operator);
                        Some(format!("{} {} {} AND {}", column, sql_op, v1, v2))
                    }
                    ValueInputMode::ListValue => {
                        if value.is_empty() {
                            return None;
                        }
                        let values: Vec<String> = value
                            .split(',')
                            .map(|s| Self::format_sql_value(s.trim(), *operator))
                            .collect();
                        Some(format!("{} {} ({})", column, sql_op, values.join(", ")))
                    }
                }
            })
            .collect();

        if conditions.is_empty() {
            String::new()
        } else {
            conditions.join(" AND ")
        }
    }

    fn format_sql_value(value: &str, operator: FilterOperator) -> String {
        match operator {
            FilterOperator::IsTrue => "TRUE".to_string(),
            FilterOperator::IsFalse => "FALSE".to_string(),
            FilterOperator::Between => value.to_string(),
            _ => match value.to_uppercase().as_str() {
                "NULL" => "NULL".to_string(),
                _ => format!("'{}'", value.replace('\'', "''")),
            },
        }
    }

    pub fn set_schema(&mut self, schema: TableSchema, _cx: &mut Context<Self>) {
        self.schema = schema.clone();

        // 如果条件行还没有初始化，标记为需要初始化
        if self.condition_rows.is_empty() {
            self.needs_init = true;
            return;
        }

        // 更新现有行的字段选择器
        for row in &mut self.condition_rows {
            let column_items: Vec<ColumnSelectItem> = schema
                .columns
                .iter()
                .map(|col| ColumnSelectItem {
                    column: col.clone(),
                })
                .collect();

            // 需要 window 来更新，但这里没有 window
            // 延迟到 render 时更新
            let _ = (column_items, row);
        }
    }
}

impl Render for TableFilterEditor {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        use gpui::{div, ParentElement};

        // 检查是否需要初始化条件行
        if self.needs_init && !self.schema.columns.is_empty() {
            self.needs_init = false;
            let row_id = self.next_row_id;
            self.next_row_id += 1;
            let row = FilterRowState::new(row_id, &self.schema, window, cx);
            self.condition_rows.push(row);
            self.setup_row_subscriptions(row_id, window, cx);

            // 初始化完成后，触发第一个列的 on_column_selected
            if let Some(first_column) = self.schema.columns.first() {
                if let Some(row) = self.condition_rows.first_mut() {
                    row.update_operators_for_column(first_column, window, cx);
                }
            }

            cx.notify();
        }

        let can_delete = self.condition_rows.len() > 1;

        // 构建条件行的迭代器
        let condition_rows_iter = self.condition_rows.iter().map(|row| {
            let row_id = row.row_id;
            let input_mode = row.current_input_mode;
            let column_select = row.column_select.clone();
            let operator_select = row.operator_select.clone();
            let value_input = row.value_input.clone();
            let value2_input = row.value2_input.clone();

            gpui::div()
                .flex()
                .flex_row()
                .items_center()
                .gap_2()
                .h(px(40.))
                .px_2()
                .border_1()
                .border_color(cx.theme().border)
                .rounded_md()
                // 添加条件按钮
                .child(
                    Button::new(("add_condition", row_id))
                        .icon(IconName::Plus)
                        .ghost()
                        .on_click(cx.listener(
                            move |this, _: &ClickEvent, window: &mut Window, cx| {
                                this.add_condition_row(window, cx);
                                cx.notify();
                            },
                        )),
                )
                // 删除条件按钮
                .child(
                    Button::new(("remove_condition", row_id))
                        .icon(IconName::Close)
                        .ghost()
                        .disabled(!can_delete)
                        .on_click(cx.listener(
                            move |this, _: &ClickEvent, _window: &mut Window, cx| {
                                this.remove_condition_row(row_id, cx);
                            },
                        )),
                )
                .child(
                    gpui::div()
                        .flex()
                        .w(px(240.))
                        .h(px(28.))
                        .content_center()
                        .overflow_hidden()
                        .child(Select::new(&column_select).small()),
                )
                .child(
                    gpui::div()
                        .flex()
                        .w(px(200.))
                        .h(px(28.))
                        .content_center()
                        .overflow_hidden()
                        .child(Select::new(&operator_select).small()),
                )
                .when(input_mode == ValueInputMode::SingleValue || input_mode == ValueInputMode::ListValue, |this| {
                    this.child(
                        gpui::div()
                            .flex_1()
                            .min_w(px(80.))
                            .h(px(28.))
                            .content_center()
                            .overflow_hidden()
                            .child(Input::new(&value_input).small()),
                    )
                })
                .when(input_mode == ValueInputMode::DualValue, |this| {
                    this.child(
                        gpui::div()
                            .flex_1()
                            .min_w(px(60.))
                            .h(px(28.))
                            .content_center()
                            .overflow_hidden()
                            .child(Input::new(&value_input)),
                    )
                    .child(
                        gpui::div()
                            .text_sm()
                            .text_color(gpui::rgb(0x666666))
                            .px_1()
                            .child("AND"),
                    )
                    .child(
                        gpui::div()
                            .flex_1()
                            .min_w(px(60.))
                            .h(px(28.))
                            .content_center()
                            .overflow_hidden()
                            .child(Input::new(&value2_input)),
                    )
                })
                // .child(
                //     Button::new(("remove_condition", row_id))
                //         .icon(IconName::Close)
                //         .ghost()
                //         .on_click(cx.listener(
                //             move |this, _: &ClickEvent, _window: &mut Window, cx| {
                //                 this.remove_condition_row(row_id);
                //                 cx.notify();
                //             },
                //         )),
                // )
        });

        gpui::div()
            .size_full()
            .flex_col()
            .gap_2()
            // WHERE 条件构建器（每行自带 + 按钮）
            .child(
                div()
                    .flex_col()
                    .gap_2()
                    // 条件行
                    .children(condition_rows_iter),
            )
    }
}

impl EventEmitter<FilterEditorEvent> for TableFilterEditor {}
