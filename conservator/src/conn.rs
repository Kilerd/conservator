//! 连接管理
//!
//! 提供基于 `deadpool-postgres` 的连接池管理

use crate::{Error, Executor};
use async_trait::async_trait;
use deadpool_postgres::{Config, Pool, Runtime};
use tokio_postgres::{types::FromSql, types::ToSql, NoTls, Row};

/// 连接池包装器
///
/// 提供便捷的方法来创建和管理 PostgreSQL 连接池
pub struct PooledConnection {
    pool: Pool,
}

impl PooledConnection {
    /// 从数据库 URL 创建连接池
    ///
    /// # Arguments
    ///
    /// * `url` - PostgreSQL 连接 URL，格式：`postgres://user:password@host:port/database`
    ///
    /// # Example
    ///
    /// ```no_run
    /// use conservator::PooledConnection;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let pool = PooledConnection::from_url("postgres://user:pass@localhost:5432/dbname")?;
    /// let mut conn = pool.get().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn from_url(url: &str) -> Result<Self, Error> {
        // 手动解析 PostgreSQL URL
        // 格式：postgres://user:password@host:port/database
        let parsed_url = url::Url::parse(url).map_err(|e| Error::UrlParse(e.to_string()))?;

        let mut config = Config::new();

        // 解析主机和端口
        if let Some(host) = parsed_url.host_str() {
            config.host = Some(host.to_string());
        }
        if let Some(port) = parsed_url.port() {
            config.port = Some(port);
        }

        // 解析用户名和密码
        if !parsed_url.username().is_empty() {
            config.user = Some(parsed_url.username().to_string());
        }
        if let Some(password) = parsed_url.password() {
            config.password = Some(password.to_string());
        }

        // 解析数据库名
        let path = parsed_url.path().trim_start_matches('/');
        if !path.is_empty() {
            config.dbname = Some(path.to_string());
        }

        let pool = config
            .create_pool(Some(Runtime::Tokio1), NoTls)
            .map_err(|e| Error::Pool(format!("Failed to create pool: {:?}", e)))?;
        Ok(Self { pool })
    }

    /// 从配置创建连接池
    ///
    /// # Arguments
    ///
    /// * `config` - `deadpool_postgres::Config` 配置对象
    ///
    /// # Example
    ///
    /// ```no_run
    /// use conservator::PooledConnection;
    /// use deadpool_postgres::Config;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut config = Config::new();
    /// config.host = Some("localhost".to_string());
    /// config.port = Some(5432);
    /// config.user = Some("postgres".to_string());
    /// config.password = Some("postgres".to_string());
    /// config.dbname = Some("mydb".to_string());
    /// config.pool = Some(deadpool_postgres::PoolConfig {
    ///     max_size: 20,
    ///     ..Default::default()
    /// });
    ///
    /// let pool = PooledConnection::from_config(config)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn from_config(config: Config) -> Result<Self, Error> {
        let pool = config
            .create_pool(Some(Runtime::Tokio1), NoTls)
            .map_err(|e| Error::Pool(format!("Failed to create pool: {:?}", e)))?;
        Ok(Self { pool })
    }

    /// 获取连接池的引用
    ///
    /// 用于直接访问底层的 `deadpool_postgres::Pool`
    pub fn pool(&self) -> &Pool {
        &self.pool
    }

    /// 获取一个连接
    ///
    /// 返回一个 `Connection`，连接在使用完毕后会自动归还到连接池
    ///
    /// # Example
    ///
    /// ```ignore
    /// use conservator::PooledConnection;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let pool = PooledConnection::from_url("postgres://user:pass@localhost:5432/dbname")?;
    /// let mut conn = pool.get().await?;
    /// // conn 在 drop 时自动归还到连接池
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get(&self) -> Result<Connection, Error> {
        let client = self
            .pool
            .get()
            .await
            .map_err(|e| Error::Pool(format!("Failed to get connection: {:?}", e)))?;

        Ok(Connection { client })
    }
}

impl AsRef<Pool> for PooledConnection {
    fn as_ref(&self) -> &Pool {
        &self.pool
    }
}

impl From<Pool> for PooledConnection {
    fn from(pool: Pool) -> Self {
        Self { pool }
    }
}

/// 数据库连接
///
/// 封装了从连接池获取的客户端连接
pub struct Connection {
    client: deadpool_postgres::Client,
}

impl Connection {
    /// 开始事务
    ///
    /// 借用当前连接，返回一个带生命周期的事务
    ///
    /// # Example
    ///
    /// ```ignore
    /// use conservator::PooledConnection;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let pool = PooledConnection::from_url("postgres://user:pass@localhost:5432/dbname")?;
    /// let mut conn = pool.get().await?;
    /// let tx = conn.begin().await?;
    /// // 执行事务操作...
    /// tx.commit().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn begin(&mut self) -> Result<Transaction<'_>, Error> {
        let tx = self.client.transaction().await?;
        Ok(Transaction { inner: tx })
    }

    /// 获取底层 client 引用
    pub fn client(&self) -> &deadpool_postgres::Client {
        &self.client
    }

    /// 获取底层 client 可变引用
    pub fn client_mut(&mut self) -> &mut deadpool_postgres::Client {
        &mut self.client
    }
}

/// 数据库事务
///
/// 封装了 PostgreSQL 事务，带有明确的生命周期
pub struct Transaction<'a> {
    inner: deadpool_postgres::Transaction<'a>,
}

impl<'a> Transaction<'a> {
    /// 提交事务
    pub async fn commit(self) -> Result<(), Error> {
        self.inner.commit().await.map_err(Error::from)
    }

    /// 回滚事务
    pub async fn rollback(self) -> Result<(), Error> {
        self.inner.rollback().await.map_err(Error::from)
    }

    /// 获取底层事务引用
    pub fn inner(&self) -> &deadpool_postgres::Transaction<'a> {
        &self.inner
    }
}

/// 为 `Connection` 实现 `Executor` trait
#[async_trait]
impl Executor for Connection {
    async fn execute(&self, query: &str, params: &[&(dyn ToSql + Sync)]) -> Result<u64, Error> {
        use std::ops::Deref;
        use tokio_postgres::GenericClient;
        let client: &tokio_postgres::Client = self.client.deref();
        let stmt = client.prepare(query).await?;
        GenericClient::execute(client, &stmt, params)
            .await
            .map_err(Error::from)
    }

    async fn query_one(&self, query: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Row, Error> {
        use std::ops::Deref;
        use tokio_postgres::GenericClient;
        let client: &tokio_postgres::Client = self.client.deref();
        let stmt = client.prepare(query).await?;
        GenericClient::query_one(client, &stmt, params)
            .await
            .map_err(Error::from)
    }

    async fn query(&self, query: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Vec<Row>, Error> {
        use std::ops::Deref;
        use tokio_postgres::GenericClient;
        let client: &tokio_postgres::Client = self.client.deref();
        let stmt = client.prepare(query).await?;
        GenericClient::query(client, &stmt, params)
            .await
            .map_err(Error::from)
    }

    async fn query_scalar<T>(&self, query: &str, params: &[&(dyn ToSql + Sync)]) -> Result<T, Error>
    where
        T: for<'r> FromSql<'r>,
    {
        let row = self.query_one(query, params).await?;
        row.try_get(0).map_err(Error::from)
    }
}

/// 为 `Transaction` 实现 `Executor` trait
#[async_trait]
impl<'a> Executor for Transaction<'a> {
    async fn execute(&self, query: &str, params: &[&(dyn ToSql + Sync)]) -> Result<u64, Error> {
        use std::ops::Deref;
        use tokio_postgres::GenericClient;
        let tx: &tokio_postgres::Transaction<'_> = self.inner.deref();
        let stmt = tx.prepare(query).await?;
        GenericClient::execute(tx, &stmt, params)
            .await
            .map_err(Error::from)
    }

    async fn query_one(&self, query: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Row, Error> {
        use std::ops::Deref;
        use tokio_postgres::GenericClient;
        let tx: &tokio_postgres::Transaction<'_> = self.inner.deref();
        let stmt = tx.prepare(query).await?;
        GenericClient::query_one(tx, &stmt, params)
            .await
            .map_err(Error::from)
    }

    async fn query(&self, query: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Vec<Row>, Error> {
        use std::ops::Deref;
        use tokio_postgres::GenericClient;
        let tx: &tokio_postgres::Transaction<'_> = self.inner.deref();
        let stmt = tx.prepare(query).await?;
        GenericClient::query(tx, &stmt, params)
            .await
            .map_err(Error::from)
    }

    async fn query_scalar<T>(&self, query: &str, params: &[&(dyn ToSql + Sync)]) -> Result<T, Error>
    where
        T: for<'r> FromSql<'r>,
    {
        let row = self.query_one(query, params).await?;
        row.try_get(0).map_err(Error::from)
    }
}

/// 为 `PooledConnection` 实现 `Executor` trait
///
/// 每次调用都会从连接池获取一个新的连接。
#[async_trait]
impl Executor for PooledConnection {
    async fn execute(&self, query: &str, params: &[&(dyn ToSql + Sync)]) -> Result<u64, Error> {
        let conn = self.get().await?;
        Executor::execute(&conn, query, params).await
    }

    async fn query_one(&self, query: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Row, Error> {
        let conn = self.get().await?;
        Executor::query_one(&conn, query, params).await
    }

    async fn query(&self, query: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Vec<Row>, Error> {
        let conn = self.get().await?;
        Executor::query(&conn, query, params).await
    }

    async fn query_scalar<T>(&self, query: &str, params: &[&(dyn ToSql + Sync)]) -> Result<T, Error>
    where
        T: for<'r> FromSql<'r>,
    {
        let conn = self.get().await?;
        Executor::query_scalar(&conn, query, params).await
    }
}

/// 为 `&PooledConnection` 实现 `Executor` trait
#[async_trait]
impl Executor for &PooledConnection {
    async fn execute(&self, query: &str, params: &[&(dyn ToSql + Sync)]) -> Result<u64, Error> {
        (*self).execute(query, params).await
    }

    async fn query_one(&self, query: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Row, Error> {
        (*self).query_one(query, params).await
    }

    async fn query(&self, query: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Vec<Row>, Error> {
        (*self).query(query, params).await
    }

    async fn query_scalar<T>(&self, query: &str, params: &[&(dyn ToSql + Sync)]) -> Result<T, Error>
    where
        T: for<'r> FromSql<'r>,
    {
        (*self).query_scalar(query, params).await
    }
}

/// 为 `&Connection` 实现 `Executor` trait
#[async_trait]
impl Executor for &Connection {
    async fn execute(&self, query: &str, params: &[&(dyn ToSql + Sync)]) -> Result<u64, Error> {
        (*self).execute(query, params).await
    }

    async fn query_one(&self, query: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Row, Error> {
        (*self).query_one(query, params).await
    }

    async fn query(&self, query: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Vec<Row>, Error> {
        (*self).query(query, params).await
    }

    async fn query_scalar<T>(&self, query: &str, params: &[&(dyn ToSql + Sync)]) -> Result<T, Error>
    where
        T: for<'r> FromSql<'r>,
    {
        (*self).query_scalar(query, params).await
    }
}

/// 为 `&Transaction` 实现 `Executor` trait
#[async_trait]
impl<'a> Executor for &Transaction<'a> {
    async fn execute(&self, query: &str, params: &[&(dyn ToSql + Sync)]) -> Result<u64, Error> {
        (*self).execute(query, params).await
    }

    async fn query_one(&self, query: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Row, Error> {
        (*self).query_one(query, params).await
    }

    async fn query(&self, query: &str, params: &[&(dyn ToSql + Sync)]) -> Result<Vec<Row>, Error> {
        (*self).query(query, params).await
    }

    async fn query_scalar<T>(&self, query: &str, params: &[&(dyn ToSql + Sync)]) -> Result<T, Error>
    where
        T: for<'r> FromSql<'r>,
    {
        (*self).query_scalar(query, params).await
    }
}
