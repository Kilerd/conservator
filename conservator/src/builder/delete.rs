use crate::{Domain, Expression, SqlResult};
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

    pub async fn execute<'e, 'c: 'e, E: 'e + sqlx::Executor<'c, Database = sqlx::Postgres>>(
        self,
        executor: E,
    ) -> Result<u64, sqlx::Error> {
        let sql_result = self.build();
        let mut query = sqlx::query(&sql_result.sql);
        for value in sql_result.values {
            query = value.bind_to_query(query);
        }
        let result = query.execute(executor).await?;
        Ok(result.rows_affected())
    }
}
