//! SQL 参数值类型
//!
//! 提供 `Value` 枚举和 `IntoValue` trait，用于类型安全地将 Rust 值绑定到 SQL 查询参数。

/// 存储 SQL 参数值的枚举
///
/// 支持常见的数据库类型，包括 PostgreSQL 特有类型
#[derive(Debug, Clone)]
pub enum Value {
    // 基础类型
    Bool(bool),
    I16(i16),
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
    String(String),
    Bytes(Vec<u8>),

    // chrono 时间类型
    NaiveDate(chrono::NaiveDate),
    NaiveTime(chrono::NaiveTime),
    NaiveDateTime(chrono::NaiveDateTime),
    DateTimeUtc(chrono::DateTime<chrono::Utc>),
    DateTimeFixed(chrono::DateTime<chrono::FixedOffset>),

    // 精确数值（使用 sqlx 内部的 BigDecimal 类型）
    BigDecimal(sqlx::types::BigDecimal),

    // UUID
    Uuid(uuid::Uuid),

    // JSON
    Json(serde_json::Value),

    /// 用于扩展其他类型
    None,
}

impl Value {
    /// 将 Value 绑定到 sqlx QueryAs 查询
    pub fn bind_to<'q, O>(
        self,
        query: sqlx::query::QueryAs<
            'q,
            sqlx::Postgres,
            O,
            <sqlx::Postgres as sqlx::database::HasArguments<'q>>::Arguments,
        >,
    ) -> sqlx::query::QueryAs<
        'q,
        sqlx::Postgres,
        O,
        <sqlx::Postgres as sqlx::database::HasArguments<'q>>::Arguments,
    >
    where
        O: for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow>,
    {
        match self {
            // 基础类型
            Value::Bool(v) => query.bind(v),
            Value::I16(v) => query.bind(v),
            Value::I32(v) => query.bind(v),
            Value::I64(v) => query.bind(v),
            Value::F32(v) => query.bind(v),
            Value::F64(v) => query.bind(v),
            Value::String(v) => query.bind(v),
            Value::Bytes(v) => query.bind(v),
            // chrono 时间类型
            Value::NaiveDate(v) => query.bind(v),
            Value::NaiveTime(v) => query.bind(v),
            Value::NaiveDateTime(v) => query.bind(v),
            Value::DateTimeUtc(v) => query.bind(v),
            Value::DateTimeFixed(v) => query.bind(v),
            // 精确数值
            Value::BigDecimal(v) => query.bind(v),
            // UUID
            Value::Uuid(v) => query.bind(v),
            // JSON
            Value::Json(v) => query.bind(v),
            // None
            Value::None => query,
        }
    }

    /// 将 Value 绑定到 sqlx Query 查询（用于 DELETE/UPDATE 等不返回行的操作）
    pub fn bind_to_query<'q>(
        self,
        query: sqlx::query::Query<
            'q,
            sqlx::Postgres,
            <sqlx::Postgres as sqlx::database::HasArguments<'q>>::Arguments,
        >,
    ) -> sqlx::query::Query<
        'q,
        sqlx::Postgres,
        <sqlx::Postgres as sqlx::database::HasArguments<'q>>::Arguments,
    > {
        match self {
            // 基础类型
            Value::Bool(v) => query.bind(v),
            Value::I16(v) => query.bind(v),
            Value::I32(v) => query.bind(v),
            Value::I64(v) => query.bind(v),
            Value::F32(v) => query.bind(v),
            Value::F64(v) => query.bind(v),
            Value::String(v) => query.bind(v),
            Value::Bytes(v) => query.bind(v),
            // chrono 时间类型
            Value::NaiveDate(v) => query.bind(v),
            Value::NaiveTime(v) => query.bind(v),
            Value::NaiveDateTime(v) => query.bind(v),
            Value::DateTimeUtc(v) => query.bind(v),
            Value::DateTimeFixed(v) => query.bind(v),
            // 精确数值
            Value::BigDecimal(v) => query.bind(v),
            // UUID
            Value::Uuid(v) => query.bind(v),
            // JSON
            Value::Json(v) => query.bind(v),
            // None
            Value::None => query,
        }
    }
}

/// 将 Rust 类型转换为 Value 的 trait
pub trait IntoValue {
    fn into_value(self) -> Value;
}

// 基础类型实现
impl IntoValue for bool {
    fn into_value(self) -> Value {
        Value::Bool(self)
    }
}

impl IntoValue for i16 {
    fn into_value(self) -> Value {
        Value::I16(self)
    }
}

impl IntoValue for i32 {
    fn into_value(self) -> Value {
        Value::I32(self)
    }
}

impl IntoValue for i64 {
    fn into_value(self) -> Value {
        Value::I64(self)
    }
}

impl IntoValue for f32 {
    fn into_value(self) -> Value {
        Value::F32(self)
    }
}

impl IntoValue for f64 {
    fn into_value(self) -> Value {
        Value::F64(self)
    }
}

impl IntoValue for String {
    fn into_value(self) -> Value {
        Value::String(self)
    }
}

impl IntoValue for &str {
    fn into_value(self) -> Value {
        Value::String(self.to_string())
    }
}

impl IntoValue for Vec<u8> {
    fn into_value(self) -> Value {
        Value::Bytes(self)
    }
}

impl<T: IntoValue + Clone> IntoValue for &T {
    fn into_value(self) -> Value {
        self.clone().into_value()
    }
}

// chrono 时间类型
impl IntoValue for chrono::NaiveDate {
    fn into_value(self) -> Value {
        Value::NaiveDate(self)
    }
}

impl IntoValue for chrono::NaiveTime {
    fn into_value(self) -> Value {
        Value::NaiveTime(self)
    }
}

impl IntoValue for chrono::NaiveDateTime {
    fn into_value(self) -> Value {
        Value::NaiveDateTime(self)
    }
}

impl IntoValue for chrono::DateTime<chrono::Utc> {
    fn into_value(self) -> Value {
        Value::DateTimeUtc(self)
    }
}

impl IntoValue for chrono::DateTime<chrono::FixedOffset> {
    fn into_value(self) -> Value {
        Value::DateTimeFixed(self)
    }
}

// 精确数值
impl IntoValue for sqlx::types::BigDecimal {
    fn into_value(self) -> Value {
        Value::BigDecimal(self)
    }
}

// UUID
impl IntoValue for uuid::Uuid {
    fn into_value(self) -> Value {
        Value::Uuid(self)
    }
}

// JSON
impl IntoValue for serde_json::Value {
    fn into_value(self) -> Value {
        Value::Json(self)
    }
}

