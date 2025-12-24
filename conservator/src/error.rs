use thiserror::Error;

/// 统一的错误类型
#[derive(Error, Debug)]
pub enum Error {
    /// tokio-postgres 错误
    #[error("PostgreSQL error: {0}")]
    Postgres(#[from] tokio_postgres::Error),

    /// deadpool-postgres 连接池错误
    #[error("Pool error: {0}")]
    Pool(#[from] deadpool_postgres::PoolError),

    /// deadpool-postgres 连接池创建错误
    #[error("Create pool error: {0}")]
    CreatePool(#[from] deadpool::managed::CreatePoolError<deadpool_postgres::ConfigError>),

    /// deadpool-postgres 配置错误
    #[error("Config error: {0}")]
    Config(#[from] deadpool_postgres::ConfigError),

    /// URL 解析错误
    #[error("URL parse error: {0}")]
    UrlParse(String),
}
