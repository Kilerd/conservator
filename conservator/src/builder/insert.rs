use crate::{Creatable, Domain, Executor, Selectable};
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
    pub async fn returning_pk<E: Executor>(
        self,
        executor: &E,
    ) -> Result<T::PrimaryKey, crate::Error>
    where
        T::PrimaryKey: for<'r> tokio_postgres::types::FromSql<'r>,
    {
        let sql = format!(
            "INSERT INTO {} {} VALUES {} RETURNING \"{}\"",
            T::TABLE_NAME,
            self.data.get_columns(),
            self.data.get_insert_sql(),
            T::PK_FIELD_NAME
        );

        let values = self.data.get_values();

        // 将 Value 转换为 ToSql 参数
        let params: Vec<Box<dyn tokio_postgres::types::ToSql + Sync + Send + 'static>> = values
            .into_iter()
            .map(|v| v.to_tokio_sql_param())
            .collect::<Result<Vec<_>, _>>()?;

        // 转换为引用数组
        let param_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = params
            .iter()
            .map(|p| p.as_ref() as &(dyn tokio_postgres::types::ToSql + Sync))
            .collect();

        executor
            .query_scalar::<T::PrimaryKey>(&sql, &param_refs)
            .await
    }

    /// 执行 INSERT 并返回完整实体
    ///
    /// 生成: `INSERT INTO table (cols) VALUES (vals) RETURNING "col1", "col2", ...`
    pub async fn returning_entity<E: Executor>(self, executor: &E) -> Result<T, crate::Error>
    where
        T: Selectable,
    {
        let sql = format!(
            "INSERT INTO {} {} VALUES {} RETURNING {}",
            T::TABLE_NAME,
            self.data.get_columns(),
            self.data.get_insert_sql(),
            returning_columns::<T>()
        );

        let values = self.data.get_values();

        // 将 Value 转换为 ToSql 参数
        let params: Vec<Box<dyn tokio_postgres::types::ToSql + Sync + Send + 'static>> = values
            .into_iter()
            .map(|v| v.to_tokio_sql_param())
            .collect::<Result<Vec<_>, _>>()?;

        // 转换为引用数组
        let param_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = params
            .iter()
            .map(|p| p.as_ref() as &(dyn tokio_postgres::types::ToSql + Sync))
            .collect();

        // 执行查询
        let row = executor.query_one(&sql, &param_refs).await?;
        T::from_row(&row)
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
    pub async fn returning_pk<E: Executor>(
        self,
        executor: &E,
    ) -> Result<Vec<T::PrimaryKey>, crate::Error>
    where
        T::PrimaryKey: for<'r> tokio_postgres::types::FromSql<'r>,
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

        // 收集所有参数值
        let mut all_values = Vec::new();
        for (idx, item) in self.data.iter().enumerate() {
            all_values.extend(item.get_batch_values(idx));
        }

        // 将 Value 转换为 ToSql 参数
        let params: Vec<Box<dyn tokio_postgres::types::ToSql + Sync + Send + 'static>> = all_values
            .into_iter()
            .map(|v| v.to_tokio_sql_param())
            .collect::<Result<Vec<_>, _>>()?;

        // 转换为引用数组
        let param_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = params
            .iter()
            .map(|p| p.as_ref() as &(dyn tokio_postgres::types::ToSql + Sync))
            .collect();

        // 执行查询
        let rows = executor.query(&sql, &param_refs).await?;
        let mut results = Vec::with_capacity(rows.len());
        for row in rows {
            results.push(row.try_get::<_, T::PrimaryKey>(0)?);
        }
        Ok(results)
    }

    /// 执行 INSERT 并返回完整实体列表
    ///
    /// 生成: `INSERT INTO table (cols) VALUES (v1), (v2), ... RETURNING "col1", "col2", ...`
    pub async fn returning_entity<E: Executor>(self, executor: &E) -> Result<Vec<T>, crate::Error>
    where
        T: Selectable,
    {
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

        // 收集所有参数值
        let mut all_values = Vec::new();
        for (idx, item) in self.data.iter().enumerate() {
            all_values.extend(item.get_batch_values(idx));
        }

        // 将 Value 转换为 ToSql 参数
        let params: Vec<Box<dyn tokio_postgres::types::ToSql + Sync + Send + 'static>> = all_values
            .into_iter()
            .map(|v| v.to_tokio_sql_param())
            .collect::<Result<Vec<_>, _>>()?;

        // 转换为引用数组
        let param_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = params
            .iter()
            .map(|p| p.as_ref() as &(dyn tokio_postgres::types::ToSql + Sync))
            .collect();

        // 执行查询
        let rows = executor.query(&sql, &param_refs).await?;
        let mut results = Vec::with_capacity(rows.len());
        for row in rows {
            results.push(T::from_row(&row)?);
        }
        Ok(results)
    }

    /// 执行 INSERT（不返回数据）
    ///
    /// 生成: `INSERT INTO table (cols) VALUES (v1), (v2), ...`
    pub async fn execute<E: Executor>(self, executor: &E) -> Result<u64, crate::Error> {
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

        // 收集所有参数值
        let mut all_values = Vec::new();
        for (idx, item) in self.data.iter().enumerate() {
            all_values.extend(item.get_batch_values(idx));
        }

        // 将 Value 转换为 ToSql 参数
        let params: Vec<Box<dyn tokio_postgres::types::ToSql + Sync + Send + 'static>> = all_values
            .into_iter()
            .map(|v| v.to_tokio_sql_param())
            .collect::<Result<Vec<_>, _>>()?;

        // 转换为引用数组
        let param_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = params
            .iter()
            .map(|p| p.as_ref() as &(dyn tokio_postgres::types::ToSql + Sync))
            .collect();

        // 执行查询
        executor.execute(&sql, &param_refs).await
    }
}
