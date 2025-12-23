mod select;
mod update;
mod delete;
mod insert;

pub use select::SelectBuilder;
pub use update::UpdateBuilder;
pub use delete::DeleteBuilder;
pub use insert::{InsertBuilder, InsertManyBuilder};

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
    fn to_sql(&self) -> &'static str {
        match self {
            Order::Asc => "ASC",
            Order::Desc => "DESC",
        }
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