mod select;
mod update;
mod delete;
mod insert;

pub use select::SelectBuilder;
pub use update::UpdateBuilder;
pub use delete::DeleteBuilder;
pub use insert::{InsertBuilder, InsertManyBuilder};

use crate::expression::FieldInfo;
use crate::Expression;

/// 排序方向
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Order {
    /// 升序
    Asc,
    /// 降序
    Desc,
}

impl Order {
    pub(crate) fn to_sql(&self) -> &'static str {
        match self {
            Order::Asc => "ASC",
            Order::Desc => "DESC",
        }
    }
}

/// 带排序方向的字段
#[derive(Debug, Clone, Copy)]
pub struct OrderedField {
    pub(crate) field: FieldInfo,
    pub(crate) order: Order,
}

impl OrderedField {
    /// 创建一个带排序方向的字段
    pub fn new(field: FieldInfo, order: Order) -> Self {
        Self { field, order }
    }
}

/// 可转换为 OrderedField 的 trait
/// 
/// 实现者:
/// - `OrderedField` - 直接使用
/// - `Field<T>` - 默认升序
/// - `FieldInfo` - 默认升序
pub trait IntoOrderedField {
    fn into_ordered_field(self) -> OrderedField;
}

impl IntoOrderedField for OrderedField {
    fn into_ordered_field(self) -> OrderedField {
        self
    }
}

impl IntoOrderedField for FieldInfo {
    fn into_ordered_field(self) -> OrderedField {
        OrderedField::new(self, Order::Asc)
    }
}

// 为 (FieldInfo, Order) 元组实现，保持向后兼容
impl IntoOrderedField for (FieldInfo, Order) {
    fn into_ordered_field(self) -> OrderedField {
        OrderedField::new(self.0, self.1)
    }
}

/// JOIN 类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JoinType {
    /// INNER JOIN
    Inner,
    /// LEFT JOIN
    Left,
    /// RIGHT JOIN
    Right,
}

impl JoinType {
    fn to_sql(&self) -> &'static str {
        match self {
            JoinType::Inner => "INNER JOIN",
            JoinType::Left => "LEFT JOIN",
            JoinType::Right => "RIGHT JOIN",
        }
    }
}

/// JOIN 子句
#[derive(Debug, Clone)]
pub struct JoinClause {
    join_type: JoinType,
    table: String,
    on: Expression,
}