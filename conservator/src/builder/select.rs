use std::marker::PhantomData;

use crate::{Domain, Expression, FieldInfo, Selectable, SqlResult, Value};

use super::{IntoOrderedField, JoinClause, JoinType, OrderedField};

/// SELECT 查询构建器
///
/// 用于构建类型安全的 SELECT 查询
///
/// # Example
/// ```ignore
/// let result = SelectBuilder::<User>::new()
///     .filter(User::COLUMNS.id.eq(1))
///     .order_by(User::COLUMNS.id)  // 默认升序
///     .order_by(User::COLUMNS.name.desc())  // 显式降序
///     .limit(10)
///     .build();
/// ```
#[derive(Debug, Clone)]
pub struct SelectBuilder<CoreDomain: Domain, Returning: Selectable = CoreDomain> {
    filter_expr: Option<Expression>,
    order_by: Vec<OrderedField>,
    limit: Option<usize>,
    offset: Option<usize>,
    group_by: Vec<FieldInfo>,
    joins: Vec<JoinClause>,
    _phantom: PhantomData<CoreDomain>,
    _returning_phantom: PhantomData<Returning>,
}

impl<T: Domain> Default for SelectBuilder<T, T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Domain> SelectBuilder<T, T> {
    /// 创建新的查询构建器
    pub fn new() -> SelectBuilder<T, T> {
        SelectBuilder {
            filter_expr: None,
            order_by: Vec::new(),
            limit: None,
            offset: None,
            group_by: Vec::new(),
            joins: Vec::new(),
            _phantom: PhantomData,
            _returning_phantom: PhantomData,
        }
    }
}

impl<T: Domain, Returning: Selectable> SelectBuilder<T, Returning> {
    pub fn returning<R: Selectable>(self) -> SelectBuilder<T, R> {
        SelectBuilder::<T, R> {
            filter_expr: self.filter_expr,
            order_by: self.order_by,
            limit: self.limit,
            offset: self.offset,
            group_by: self.group_by,
            joins: self.joins,
            _phantom: self._phantom,
            _returning_phantom: PhantomData,
        }
    }

    /// 添加 WHERE 条件
    ///
    /// 多次调用会用 AND 组合条件
    pub fn filter(mut self, expr: Expression) -> Self {
        self.filter_expr = match self.filter_expr {
            Some(existing) => Some(existing & expr),
            None => Some(expr),
        };
        self
    }

    /// 添加 ORDER BY 子句
    ///
    /// 支持三种用法:
    /// - `.order_by(field)` - 默认升序
    /// - `.order_by(field.asc())` - 显式升序
    /// - `.order_by(field.desc())` - 显式降序
    ///
    /// # Example
    /// ```ignore
    /// User::select()
    ///     .order_by(User::COLUMNS.score.desc())
    ///     .order_by(User::COLUMNS.name)  // 默认升序
    ///     .all(&pool)
    /// ```
    pub fn order_by<F: IntoOrderedField>(mut self, field: F) -> Self {
        self.order_by.push(field.into_ordered_field());
        self
    }

    /// 设置 LIMIT
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// 设置 OFFSET
    pub fn offset(mut self, offset: usize) -> Self {
        self.offset = Some(offset);
        self
    }

    /// 添加 GROUP BY 子句
    pub fn group_by<F>(mut self, field: F) -> Self
    where
        F: Into<FieldInfo>,
    {
        self.group_by.push(field.into());
        self
    }

    /// 添加 INNER JOIN
    pub fn join(mut self, table: &str, on: Expression) -> Self {
        self.joins.push(JoinClause {
            join_type: JoinType::Inner,
            table: table.to_string(),
            on,
        });
        self
    }

    /// 添加 LEFT JOIN
    pub fn left_join(mut self, table: &str, on: Expression) -> Self {
        self.joins.push(JoinClause {
            join_type: JoinType::Left,
            table: table.to_string(),
            on,
        });
        self
    }

    /// 添加 RIGHT JOIN
    pub fn right_join(mut self, table: &str, on: Expression) -> Self {
        self.joins.push(JoinClause {
            join_type: JoinType::Right,
            table: table.to_string(),
            on,
        });
        self
    }

    /// 构建完整的 SQL 查询
    ///
    /// 返回包含 SQL 字符串和参数值的 SqlResult
    pub fn build(self) -> SqlResult {
        let mut sql_parts = Vec::new();
        let mut all_values: Vec<Value> = Vec::new();
        let mut param_idx = 1usize;

        // SELECT 子句 - 使用 Returning 的列名
        let columns = Returning::COLUMN_NAMES
            .iter()
            .map(|name| format!("\"{}\"", name))
            .collect::<Vec<_>>()
            .join(", ");
        sql_parts.push(format!("SELECT {} FROM {}", columns, T::TABLE_NAME));

        // JOIN 子句
        for join in self.joins {
            let (on_sql, on_values, next_idx) = join.on.build_with_offset(param_idx);
            sql_parts.push(format!(
                "{} {} ON {}",
                join.join_type.to_sql(),
                join.table,
                on_sql
            ));
            all_values.extend(on_values);
            param_idx = next_idx;
        }

        // WHERE 子句
        if let Some(expr) = self.filter_expr {
            let (where_sql, where_values, next_idx) = expr.build_with_offset(param_idx);
            sql_parts.push(format!("WHERE {}", where_sql));
            all_values.extend(where_values);
            param_idx = next_idx;
        }

        // GROUP BY 子句
        if !self.group_by.is_empty() {
            let group_by_cols = self
                .group_by
                .iter()
                .map(|f| f.quoted_name())
                .collect::<Vec<_>>()
                .join(", ");
            sql_parts.push(format!("GROUP BY {}", group_by_cols));
        }

        // ORDER BY 子句
        if !self.order_by.is_empty() {
            let order_by_cols = self
                .order_by
                .iter()
                .map(|of| format!("{} {}", of.field.quoted_name(), of.order.to_sql()))
                .collect::<Vec<_>>()
                .join(", ");
            sql_parts.push(format!("ORDER BY {}", order_by_cols));
        }

        // LIMIT 子句
        if let Some(limit) = self.limit {
            sql_parts.push(format!("LIMIT {}", limit));
        }

        // OFFSET 子句
        if let Some(offset) = self.offset {
            sql_parts.push(format!("OFFSET {}", offset));
        }

        let _ = param_idx; // 消除未使用警告

        SqlResult {
            sql: sql_parts.join(" "),
            values: all_values,
        }
    }

    /// 执行查询并返回单个结果
    pub async fn one<'e, 'c: 'e, E: 'e + sqlx::Executor<'c, Database = sqlx::Postgres>>(
        self,
        executor: E,
    ) -> Result<Returning, sqlx::Error>
    where
        Returning: for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin,
    {
        let sql_result = self.build();
        let mut query = sqlx::query_as::<_, Returning>(&sql_result.sql);
        for value in sql_result.values {
            query = value.bind_to(query);
        }
        query.fetch_one(executor).await
    }

    /// 执行查询并返回所有结果
    pub async fn all<'e, 'c: 'e, E: 'e + sqlx::Executor<'c, Database = sqlx::Postgres>>(
        self,
        executor: E,
    ) -> Result<Vec<Returning>, sqlx::Error>
    where
        Returning: for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin,
    {
        let sql_result = self.build();
        let mut query = sqlx::query_as::<_, Returning>(&sql_result.sql);
        for value in sql_result.values {
            query = value.bind_to(query);
        }
        query.fetch_all(executor).await
    }

    /// 执行查询并返回可选结果
    pub async fn optional<'e, 'c: 'e, E: 'e + sqlx::Executor<'c, Database = sqlx::Postgres>>(
        self,
        executor: E,
    ) -> Result<Option<Returning>, sqlx::Error>
    where
        Returning: for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin,
    {
        let sql_result = self.build();
        let mut query = sqlx::query_as::<_, Returning>(&sql_result.sql);
        for value in sql_result.values {
            query = value.bind_to(query);
        }
        query.fetch_optional(executor).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expression::{Expression, Operator};
    use crate::Value;

    // 模拟一个 Domain 实现用于测试
    struct TestUser {
        #[allow(dead_code)]
        id: i32,
        #[allow(dead_code)]
        name: String,
        #[allow(dead_code)]
        email: String,
    }

    impl Selectable for TestUser {
        const COLUMN_NAMES: &'static [&'static str] = &["id", "name", "email"];
    }

    impl<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> for TestUser {
        fn from_row(row: &'r sqlx::postgres::PgRow) -> Result<Self, sqlx::Error> {
            use sqlx::Row;
            Ok(Self {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                email: row.try_get("email")?,
            })
        }
    }

    #[async_trait::async_trait]
    impl Domain for TestUser {
        const PK_FIELD_NAME: &'static str = "id";
        const TABLE_NAME: &'static str = "users";

        type PrimaryKey = i32;

        async fn update<'e, 'c: 'e, E: 'e + sqlx::Executor<'c, Database = sqlx::Postgres>>(
            &self,
            _executor: E,
        ) -> Result<(), sqlx::Error> {
            unimplemented!()
        }
    }

    fn id_field() -> FieldInfo {
        FieldInfo::new("id", "users", true)
    }

    fn name_field() -> FieldInfo {
        FieldInfo::new("name", "users", false)
    }

    #[test]
    fn test_simple_select() {
        let result = SelectBuilder::<TestUser>::new().build();
        assert_eq!(result.sql, "SELECT \"id\", \"name\", \"email\" FROM users");
        assert!(result.values.is_empty());
    }

    #[test]
    fn test_select_with_filter() {
        let expr = Expression::comparison(id_field(), Operator::Eq, Value::I32(1));
        let result = SelectBuilder::<TestUser>::new().filter(expr).build();
        assert_eq!(
            result.sql,
            "SELECT \"id\", \"name\", \"email\" FROM users WHERE \"id\" = $1"
        );
        assert_eq!(result.values.len(), 1);
    }

    #[test]
    fn test_select_with_order_by() {
        let result = SelectBuilder::<TestUser>::new()
            .order_by(id_field()) // 默认升序
            .build();
        assert_eq!(
            result.sql,
            "SELECT \"id\", \"name\", \"email\" FROM users ORDER BY \"id\" ASC"
        );
    }

    #[test]
    fn test_select_with_limit_offset() {
        let result = SelectBuilder::<TestUser>::new()
            .limit(10)
            .offset(20)
            .build();
        assert_eq!(
            result.sql,
            "SELECT \"id\", \"name\", \"email\" FROM users LIMIT 10 OFFSET 20"
        );
    }

    #[test]
    fn test_select_with_group_by() {
        let result = SelectBuilder::<TestUser>::new()
            .group_by(name_field())
            .build();
        assert_eq!(
            result.sql,
            "SELECT \"id\", \"name\", \"email\" FROM users GROUP BY \"name\""
        );
    }

    #[test]
    fn test_complex_select() {
        use crate::builder::{Order, OrderedField};

        let expr = Expression::comparison(id_field(), Operator::Gt, Value::I32(10));
        let result = SelectBuilder::<TestUser>::new()
            .filter(expr)
            .order_by(OrderedField::new(name_field(), Order::Desc))
            .limit(50)
            .offset(100)
            .build();
        assert_eq!(
            result.sql,
            "SELECT \"id\", \"name\", \"email\" FROM users WHERE \"id\" > $1 ORDER BY \"name\" DESC LIMIT 50 OFFSET 100"
        );
        assert_eq!(result.values.len(), 1);
    }
}
