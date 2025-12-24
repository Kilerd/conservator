use crate::{Domain, Executor, Expression, SqlResult, Value};
use std::marker::PhantomData;

pub struct DeleteBuilder<T: Domain, const FILTER_SET: bool = false> {
    filter_expr: Option<Expression>,
    _phantom: PhantomData<T>,
}

impl<T: Domain, const FILTER_SET: bool> Default for DeleteBuilder<T, FILTER_SET> {
    fn default() -> Self {
        Self {
            filter_expr: None,
            _phantom: PhantomData,
        }
    }
}

impl<T: Domain, const FILTER_SET: bool> DeleteBuilder<T, FILTER_SET> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn filter(self, expr: Expression) -> DeleteBuilder<T, true> {
        let updated_expr = match self.filter_expr {
            Some(filter_expr) => filter_expr & expr,
            None => expr,
        };
        DeleteBuilder::<T, true> {
            filter_expr: Some(updated_expr),
            _phantom: self._phantom,
        }
    }
}

impl<T: Domain> DeleteBuilder<T, true> {
    pub fn build(self) -> SqlResult {
        let mut sql = String::new();
        sql.push_str("DELETE FROM ");
        sql.push_str(T::TABLE_NAME);

        let values = if let Some(filter_expr) = self.filter_expr {
            let result = filter_expr.build();
            sql.push_str(" WHERE ");
            sql.push_str(&result.sql);
            result.values
        } else {
            Vec::new()
        };

        SqlResult { sql, values }
    }

    pub async fn execute<E: Executor>(self, executor: &E) -> Result<u64, crate::Error> {
        let sql_result = self.build();

        // 将 Value 转换为 ToSql 参数
        let params: Vec<Box<dyn tokio_postgres::types::ToSql + Sync + Send + 'static>> = sql_result
            .values
            .into_iter()
            .map(|v: Value| v.to_tokio_sql_param())
            .collect::<Result<Vec<_>, _>>()?;

        // 转换为引用数组
        let param_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = params
            .iter()
            .map(|p| p.as_ref() as &(dyn tokio_postgres::types::ToSql + Sync))
            .collect();

        // 执行查询
        executor.execute(&sql_result.sql, &param_refs).await
    }
}
