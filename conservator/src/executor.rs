//! 执行器抽象层
//!
//! 提供统一的数据库执行接口，支持 `tokio_postgres::Client` 和 `deadpool_postgres::Client`

use crate::Error;
use async_trait::async_trait;
use tokio_postgres::{Row, types::FromSql, types::ToSql};

/// 统一的数据库执行器 trait
///
/// 此 trait 抽象了不同 PostgreSQL 客户端的执行接口，允许统一使用 `tokio_postgres::Client`
/// 和 `deadpool_postgres::Client`。
#[async_trait]
pub trait Executor: Send + Sync {
    /// 执行一个不返回行的 SQL 语句（如 INSERT、UPDATE、DELETE）
    ///
    /// # Arguments
    ///
    /// * `query` - SQL 查询字符串
    /// * `params` - 查询参数
    ///
    /// # Returns
    ///
    /// 返回受影响的行数
    async fn execute(&self, query: &str, params: &[&(dyn ToSql + Sync)]) -> Result<u64, Error>;

    /// 执行一个返回单行的 SQL 查询
    ///
    /// # Arguments
    ///
    /// * `query` - SQL 查询字符串
    /// * `params` - 查询参数
    ///
    /// # Returns
    ///
    /// 返回单行结果，如果查询返回多行或没有行，则返回错误
    async fn query_one(&self, query: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Row, Error>;

    /// 执行一个返回多行的 SQL 查询
    ///
    /// # Arguments
    ///
    /// * `query` - SQL 查询字符串
    /// * `params` - 查询参数
    ///
    /// # Returns
    ///
    /// 返回所有行的结果
    async fn query(&self, query: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Vec<Row>, Error>;

    /// 执行一个返回标量值的 SQL 查询
    ///
    /// # Arguments
    ///
    /// * `query` - SQL 查询字符串
    /// * `params` - 查询参数
    ///
    /// # Returns
    ///
    /// 返回第一行第一列的值
    async fn query_scalar<T>(
        &self,
        query: &str,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<T, Error>
    where
        T: for<'r> FromSql<'r>;

    /// 执行一个返回可选行的 SQL 查询
    ///
    /// # Arguments
    ///
    /// * `query` - SQL 查询字符串
    /// * `params` - 查询参数
    ///
    /// # Returns
    ///
    /// 返回 Some(Row) 如果查询返回一行，None 如果没有行，如果返回多行则返回错误
    async fn query_opt(
        &self,
        query: &str,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<Option<Row>, Error>;
}

/// 为 `tokio_postgres::Client` 实现 `Executor` trait
#[async_trait]
impl Executor for tokio_postgres::Client {
    async fn execute(&self, query: &str, params: &[&(dyn ToSql + Sync)]) -> Result<u64, Error> {
        use tokio_postgres::GenericClient;
        let stmt = self.prepare(query).await?;
        GenericClient::execute(self, &stmt, params)
            .await
            .map_err(Error::from)
    }

    async fn query_one(&self, query: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Row, Error> {
        use tokio_postgres::GenericClient;
        let stmt = self.prepare(query).await?;
        GenericClient::query_one(self, &stmt, params)
            .await
            .map_err(Error::from)
    }

    async fn query(&self, query: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Vec<Row>, Error> {
        use tokio_postgres::GenericClient;
        let stmt = self.prepare(query).await?;
        GenericClient::query(self, &stmt, params)
            .await
            .map_err(Error::from)
    }

    async fn query_scalar<T>(&self, query: &str, params: &[&(dyn ToSql + Sync)]) -> Result<T, Error>
    where
        T: for<'r> FromSql<'r>,
    {
        use tokio_postgres::GenericClient;
        let stmt = self.prepare(query).await?;
        let row = GenericClient::query_one(self, &stmt, params).await?;
        row.try_get(0).map_err(Error::from)
    }

    async fn query_opt(
        &self,
        query: &str,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<Option<Row>, Error> {
        use tokio_postgres::GenericClient;
        let stmt = self.prepare(query).await?;
        let rows = GenericClient::query(self, &stmt, params).await?;
        match rows.len() {
            0 => Ok(None),
            1 => Ok(Some(rows.into_iter().next().unwrap())),
            _ => {
                // Return multiple rows error by calling query_one
                self.query_one(query, params).await?;
                unreachable!()
            }
        }
    }
}

/// 为 `deadpool_postgres::Transaction` 实现 `Executor` trait
///
/// `deadpool_postgres::Transaction` 通过 `Deref` 实现为 `tokio_postgres::Transaction`，
/// 所以我们可以直接调用底层的方法。
#[async_trait]
impl<'a> Executor for deadpool_postgres::Transaction<'a> {
    async fn execute(&self, query: &str, params: &[&(dyn ToSql + Sync)]) -> Result<u64, Error> {
        // 通过 Deref 访问 tokio_postgres::Transaction
        Executor::execute(self as &tokio_postgres::Transaction, query, params).await
    }

    async fn query_one(&self, query: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Row, Error> {
        Executor::query_one(self as &tokio_postgres::Transaction, query, params).await
    }

    async fn query(&self, query: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Vec<Row>, Error> {
        Executor::query(self as &tokio_postgres::Transaction, query, params).await
    }

    async fn query_scalar<T>(&self, query: &str, params: &[&(dyn ToSql + Sync)]) -> Result<T, Error>
    where
        T: for<'r> FromSql<'r>,
    {
        Executor::query_scalar(self as &tokio_postgres::Transaction, query, params).await
    }

    async fn query_opt(
        &self,
        query: &str,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<Option<Row>, Error> {
        Executor::query_opt(self as &tokio_postgres::Transaction, query, params).await
    }
}

/// 为 `tokio_postgres::Transaction` 实现 `Executor` trait
#[async_trait]
impl<'a> Executor for tokio_postgres::Transaction<'a> {
    async fn execute(&self, query: &str, params: &[&(dyn ToSql + Sync)]) -> Result<u64, Error> {
        use tokio_postgres::GenericClient;
        let stmt = self.prepare(query).await?;
        GenericClient::execute(self, &stmt, params)
            .await
            .map_err(Error::from)
    }

    async fn query_one(&self, query: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Row, Error> {
        use tokio_postgres::GenericClient;
        let stmt = self.prepare(query).await?;
        GenericClient::query_one(self, &stmt, params)
            .await
            .map_err(Error::from)
    }

    async fn query(&self, query: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Vec<Row>, Error> {
        use tokio_postgres::GenericClient;
        let stmt = self.prepare(query).await?;
        GenericClient::query(self, &stmt, params)
            .await
            .map_err(Error::from)
    }

    async fn query_scalar<T>(&self, query: &str, params: &[&(dyn ToSql + Sync)]) -> Result<T, Error>
    where
        T: for<'r> FromSql<'r>,
    {
        use tokio_postgres::GenericClient;
        let stmt = self.prepare(query).await?;
        let row = GenericClient::query_one(self, &stmt, params).await?;
        row.try_get(0).map_err(Error::from)
    }

    async fn query_opt(
        &self,
        query: &str,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<Option<Row>, Error> {
        use tokio_postgres::GenericClient;
        let stmt = self.prepare(query).await?;
        let rows = GenericClient::query(self, &stmt, params).await?;
        match rows.len() {
            0 => Ok(None),
            1 => Ok(Some(rows.into_iter().next().unwrap())),
            _ => {
                // Return multiple rows error by calling query_one
                Executor::query_one(self as &tokio_postgres::Transaction, query, params).await?;
                unreachable!()
            }
        }
    }
}

/// 为 `deadpool_postgres::Client` 实现 `Executor` trait
///
/// `deadpool_postgres::Client` 通过 `Deref` 实现为 `tokio_postgres::Client`，
/// 所以我们可以直接调用底层的方法。
#[async_trait]
impl Executor for deadpool_postgres::Client {
    async fn execute(&self, query: &str, params: &[&(dyn ToSql + Sync)]) -> Result<u64, Error> {
        // deadpool_postgres::Client 通过 Deref 实现为 tokio_postgres::Client
        // 所以我们可以直接调用 prepare 和 execute
        let stmt = self.prepare(query).await?;
        // 使用完全限定的方法调用避免递归
        <tokio_postgres::Client as tokio_postgres::GenericClient>::execute(self, &stmt, params)
            .await
            .map_err(Error::from)
    }

    async fn query_one(&self, query: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Row, Error> {
        let stmt = self.prepare(query).await?;
        <tokio_postgres::Client as tokio_postgres::GenericClient>::query_one(self, &stmt, params)
            .await
            .map_err(Error::from)
    }

    async fn query(&self, query: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Vec<Row>, Error> {
        let stmt = self.prepare(query).await?;
        <tokio_postgres::Client as tokio_postgres::GenericClient>::query(self, &stmt, params)
            .await
            .map_err(Error::from)
    }

    async fn query_scalar<T>(&self, query: &str, params: &[&(dyn ToSql + Sync)]) -> Result<T, Error>
    where
        T: for<'r> FromSql<'r>,
    {
        let stmt = self.prepare(query).await?;
        let row = <tokio_postgres::Client as tokio_postgres::GenericClient>::query_one(
            self, &stmt, params,
        )
        .await?;
        row.try_get(0).map_err(Error::from)
    }

    async fn query_opt(
        &self,
        query: &str,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<Option<Row>, Error> {
        let stmt = self.prepare(query).await?;
        let rows =
            <tokio_postgres::Client as tokio_postgres::GenericClient>::query(self, &stmt, params)
                .await?;
        match rows.len() {
            0 => Ok(None),
            1 => Ok(Some(rows.into_iter().next().unwrap())),
            _ => {
                // Return multiple rows error by calling query_one
                self.query_one(query, params).await?;
                unreachable!()
            }
        }
    }
}
