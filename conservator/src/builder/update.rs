use crate::{Domain, Executor, Expression, FieldInfo, SqlResult, Value};
use std::marker::PhantomData;

/// UPDATE 查询构建器
///
/// 使用类型状态模式确保必须同时设置了 SET 和 FILTER 才能执行 update 操作
///
/// # Example
/// ```ignore
/// let result = User::update()
///     .set(User::COLUMNS.name, "new_name".to_string())
///     .set(User::COLUMNS.email, "new@email.com".to_string())
///     .filter(User::COLUMNS.id.eq(1))
///     .build();
/// ```
pub struct UpdateBuilder<
    T: Domain,
    const SET_CALLED: bool = false,
    const FILTER_CALLED: bool = false,
> {
    /// 要更新的字段和值
    updates: Vec<(FieldInfo, Value)>,
    /// WHERE 条件
    filter_expr: Option<Expression>,
    _phantom: PhantomData<T>,
}

impl<T: Domain> Default for UpdateBuilder<T, false, false> {
    fn default() -> Self {
        Self {
            updates: Vec::new(),
            filter_expr: None,
            _phantom: PhantomData,
        }
    }
}

impl<T: Domain> UpdateBuilder<T, false, false> {
    pub fn new() -> Self {
        Self::default()
    }
}

impl<T: Domain, const SET_CALLED: bool, const FILTER_CALLED: bool>
    UpdateBuilder<T, SET_CALLED, FILTER_CALLED>
{
    pub fn set<V: crate::IntoValue>(
        self,
        field: crate::Field<V>,
        value: V,
    ) -> UpdateBuilder<T, true, FILTER_CALLED> {
        let mut updates = self.updates;
        updates.push((field.info(), value.into_value()));
        UpdateBuilder::<T, true, FILTER_CALLED> {
            updates,
            filter_expr: self.filter_expr,
            _phantom: self._phantom,
        }
    }
    /// 添加额外的 WHERE 条件（AND 组合）
    pub fn filter(self, expr: Expression) -> UpdateBuilder<T, SET_CALLED, true> {
        let updated_expr = match self.filter_expr {
            Some(filter_expr) => filter_expr & expr,
            None => expr,
        };
        UpdateBuilder::<T, SET_CALLED, true> {
            updates: self.updates,
            filter_expr: Some(updated_expr),
            _phantom: self._phantom,
        }
    }
}

// 只有 SET_CALLED = true 且 FILTER_CALLED = true 时才能 build 和 execute
impl<T: Domain> UpdateBuilder<T, true, true> {
    /// 构建 SQL 语句
    pub fn build(self) -> SqlResult {
        let mut sql = String::new();
        sql.push_str("UPDATE ");
        sql.push_str(T::TABLE_NAME);
        sql.push_str(" SET ");

        let mut values = Vec::new();
        let mut param_idx = 1;

        // 构建 SET 子句
        for (i, (field, value)) in self.updates.into_iter().enumerate() {
            if i > 0 {
                sql.push_str(", ");
            }
            sql.push_str(&format!("\"{}\" = ${}", field.name, param_idx));
            param_idx += 1;
            values.push(value);
        }

        // 构建 WHERE 子句
        if let Some(filter_expr) = self.filter_expr {
            let (filter_sql, filter_values, _) = filter_expr.build_with_offset(param_idx);
            sql.push_str(" WHERE ");
            sql.push_str(&filter_sql);
            values.extend(filter_values);
        }

        SqlResult { sql, values }
    }

    /// 执行 UPDATE 语句
    pub async fn execute<E: Executor>(self, executor: &E) -> Result<u64, crate::Error> {
        let sql_result = self.build();
        let prepared = super::PreparedParams::new(sql_result.values)?;
        let param_refs = prepared.as_params();

        // 执行查询
        executor.execute(&sql_result.sql, &param_refs).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Selectable;

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

        fn from_row(row: &crate::Row) -> Result<Self, crate::Error> {
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

        async fn save<E: crate::Executor>(&self, _executor: &E) -> Result<(), crate::Error> {
            unimplemented!()
        }
    }

    fn id_field() -> crate::Field<i32> {
        crate::Field::new("id", "users", true)
    }

    fn name_field() -> crate::Field<String> {
        crate::Field::new("name", "users", false)
    }

    fn email_field() -> crate::Field<String> {
        crate::Field::new("email", "users", false)
    }

    #[test]
    fn test_update_single_field() {
        let result = UpdateBuilder::<TestUser>::new()
            .set(name_field(), "new_name".to_string())
            .filter(id_field().eq(1))
            .build();

        assert_eq!(
            result.sql,
            "UPDATE users SET \"name\" = $1 WHERE \"id\" = $2"
        );
        assert_eq!(result.values.len(), 2);
    }

    #[test]
    fn test_update_multiple_fields() {
        let result = UpdateBuilder::<TestUser>::new()
            .set(name_field(), "new_name".to_string())
            .set(email_field(), "new@email.com".to_string())
            .filter(id_field().eq(1))
            .build();

        assert_eq!(
            result.sql,
            "UPDATE users SET \"name\" = $1, \"email\" = $2 WHERE \"id\" = $3"
        );
        assert_eq!(result.values.len(), 3);
    }

    #[test]
    fn test_update_with_complex_filter() {
        let result = UpdateBuilder::<TestUser>::new()
            .set(name_field(), "new_name".to_string())
            .filter(id_field().eq(1) & name_field().eq("old_name".to_string()))
            .build();

        assert!(result.sql.contains("UPDATE users SET"));
        assert!(result.sql.contains("WHERE"));
        assert_eq!(result.values.len(), 3);
    }

    #[test]
    fn test_filter_before_set() {
        // 先 filter 再 set 也应该可以
        let result = UpdateBuilder::<TestUser>::new()
            .filter(id_field().eq(1))
            .set(name_field(), "new_name".to_string())
            .build();

        assert_eq!(
            result.sql,
            "UPDATE users SET \"name\" = $1 WHERE \"id\" = $2"
        );
        assert_eq!(result.values.len(), 2);
    }
}
