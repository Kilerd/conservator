use crate::{Creatable, Domain};
use std::marker::PhantomData;

/// INSERT 查询构建器
///
/// 用于构建 INSERT 语句并支持不同的返回类型
///
/// # Example
/// ```ignore
/// // 返回主键
/// let pk = CreateUser { name: "test".into(), email: "a@b.com".into() }
///     .insert::<User>()
///     .returning_pk(db)
///     .await?;
///
/// // 返回完整实体
/// let user = CreateUser { name: "test".into(), email: "a@b.com".into() }
///     .insert::<User>()
///     .returning_entity(db)
///     .await?;
/// ```
/// 构建返回列名的 SQL 片段
fn returning_columns<T: Domain>() -> String {
    T::COLUMN_NAMES
        .iter()
        .map(|name| format!("\"{}\"", name))
        .collect::<Vec<_>>()
        .join(", ")
}

pub struct InsertBuilder<T: Domain, C: Creatable> {
    data: C,
    _phantom: PhantomData<T>,
}

impl<T: Domain, C: Creatable> InsertBuilder<T, C> {
    pub fn new(data: C) -> Self {
        Self {
            data,
            _phantom: PhantomData,
        }
    }

    /// 执行 INSERT 并返回主键
    ///
    /// 生成: `INSERT INTO table (cols) VALUES (vals) RETURNING "pk_field"`
    pub async fn returning_pk<'e, 'c: 'e, E: 'e + sqlx::Executor<'c, Database = sqlx::Postgres>>(
        self,
        executor: E,
    ) -> Result<T::PrimaryKey, sqlx::Error>
    where
        T::PrimaryKey:
            for<'r> sqlx::Decode<'r, sqlx::Postgres> + sqlx::Type<sqlx::Postgres> + Unpin,
    {
        let sql = format!(
            "INSERT INTO {} {} VALUES {} RETURNING \"{}\"",
            T::TABLE_NAME,
            self.data.get_columns(),
            self.data.get_insert_sql(),
            T::PK_FIELD_NAME
        );

        let query = sqlx::query_scalar(&sql);
        let query = self.data.bind_to_query_scalar(query);
        query.fetch_one(executor).await
    }

    /// 执行 INSERT 并返回完整实体
    ///
    /// 生成: `INSERT INTO table (cols) VALUES (vals) RETURNING "col1", "col2", ...`
    pub async fn returning_entity<
        'e,
        'c: 'e,
        E: 'e + sqlx::Executor<'c, Database = sqlx::Postgres>,
    >(
        self,
        executor: E,
    ) -> Result<T, sqlx::Error> {
        let sql = format!(
            "INSERT INTO {} {} VALUES {} RETURNING {}",
            T::TABLE_NAME,
            self.data.get_columns(),
            self.data.get_insert_sql(),
            returning_columns::<T>()
        );

        let query = sqlx::query_as(&sql);
        let query = self.data.build_for_query_as(query);
        query.fetch_one(executor).await
    }
}

pub struct InsertManyBuilder<T: Domain, C: Creatable> {
    data: Vec<C>,
    _phantom: PhantomData<T>,
}

impl<T: Domain, C: Creatable> InsertManyBuilder<T, C> {
    pub fn new(data: Vec<C>) -> Self {
        Self {
            data,
            _phantom: PhantomData,
        }
    }

    /// 构建批量 VALUES 子句
    fn build_values_sql(&self) -> String {
        self.data
            .iter()
            .enumerate()
            .map(|(idx, item)| item.get_batch_insert_sql(idx))
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// 执行 INSERT 并返回主键列表
    ///
    /// 生成: `INSERT INTO table (cols) VALUES (v1), (v2), ... RETURNING "pk_field"`
    pub async fn returning_pk<'e, 'c: 'e, E: 'e + sqlx::Executor<'c, Database = sqlx::Postgres>>(
        self,
        executor: E,
    ) -> Result<Vec<T::PrimaryKey>, sqlx::Error>
    where
        T::PrimaryKey:
            for<'r> sqlx::Decode<'r, sqlx::Postgres> + sqlx::Type<sqlx::Postgres> + Unpin,
    {
        if self.data.is_empty() {
            return Ok(Vec::new());
        }

        let columns = self.data[0].get_columns();
        let values_sql = self.build_values_sql();

        let sql = format!(
            "INSERT INTO {} {} VALUES {} RETURNING \"{}\"",
            T::TABLE_NAME,
            columns,
            values_sql,
            T::PK_FIELD_NAME
        );

        let mut query = sqlx::query_scalar(&sql);
        for item in self.data {
            query = item.bind_to_query_scalar(query);
        }
        query.fetch_all(executor).await
    }

    /// 执行 INSERT 并返回完整实体列表
    ///
    /// 生成: `INSERT INTO table (cols) VALUES (v1), (v2), ... RETURNING "col1", "col2", ...`
    pub async fn returning_entity<
        'e,
        'c: 'e,
        E: 'e + sqlx::Executor<'c, Database = sqlx::Postgres>,
    >(
        self,
        executor: E,
    ) -> Result<Vec<T>, sqlx::Error> {
        if self.data.is_empty() {
            return Ok(Vec::new());
        }

        let columns = self.data[0].get_columns();
        let values_sql = self.build_values_sql();

        let sql = format!(
            "INSERT INTO {} {} VALUES {} RETURNING {}",
            T::TABLE_NAME,
            columns,
            values_sql,
            returning_columns::<T>()
        );

        let mut query = sqlx::query_as(&sql);
        for item in self.data {
            query = item.build_for_query_as(query);
        }
        query.fetch_all(executor).await
    }

    /// 执行 INSERT（不返回数据）
    ///
    /// 生成: `INSERT INTO table (cols) VALUES (v1), (v2), ...`
    pub async fn execute<'e, 'c: 'e, E: 'e + sqlx::Executor<'c, Database = sqlx::Postgres>>(
        self,
        executor: E,
    ) -> Result<u64, sqlx::Error> {
        if self.data.is_empty() {
            return Ok(0);
        }

        let columns = self.data[0].get_columns();
        let values_sql = self.build_values_sql();

        let sql = format!(
            "INSERT INTO {} {} VALUES {}",
            T::TABLE_NAME,
            columns,
            values_sql
        );

        let mut query = sqlx::query(&sql);
        for item in self.data {
            query = item.build_for_query(query);
        }
        let result = query.execute(executor).await?;
        Ok(result.rows_affected())
    }
}
