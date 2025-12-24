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

    // 精确数值
    BigDecimal(bigdecimal::BigDecimal),

    // UUID
    Uuid(uuid::Uuid),

    // JSON
    Json(serde_json::Value),

    /// 用于扩展其他类型
    None,
}

impl Value {
    /// 将 Value 转换为 tokio-postgres 的 ToSql 参数
    ///
    /// 注意：tokio-postgres 需要启用相应的 feature flags 才能支持某些类型
    /// 返回一个拥有所有权的类型，可以转换为 ToSql
    pub fn to_tokio_sql_param(
        self,
    ) -> Result<Box<dyn tokio_postgres::types::ToSql + Sync + Send + 'static>, crate::Error> {
        match self {
            Value::Bool(v) => Ok(Box::new(v)),
            Value::I16(v) => Ok(Box::new(v)),
            Value::I32(v) => Ok(Box::new(v)),
            Value::I64(v) => Ok(Box::new(v)),
            Value::F32(v) => Ok(Box::new(v)),
            Value::F64(v) => Ok(Box::new(v)),
            Value::String(v) => Ok(Box::new(v)),
            Value::Bytes(v) => Ok(Box::new(v)),
            // chrono 类型：启用 with-chrono-0_4 feature 后可直接使用
            Value::NaiveDate(v) => Ok(Box::new(v)),
            Value::NaiveTime(v) => Ok(Box::new(v)),
            Value::NaiveDateTime(v) => Ok(Box::new(v)),
            Value::DateTimeUtc(v) => Ok(Box::new(v)),
            Value::DateTimeFixed(v) => Ok(Box::new(v)),
            // BigDecimal：tokio-postgres 不支持 bigdecimal feature，转换为字符串
            Value::BigDecimal(v) => Ok(Box::new(v.to_string())),
            // UUID：启用 with-uuid-1 feature 后可直接使用
            Value::Uuid(v) => Ok(Box::new(v)),
            // JSON：启用 with-serde_json-1 feature 后可直接使用
            Value::Json(v) => Ok(Box::new(v)),
            Value::None => Ok(Box::new(Option::<String>::None)),
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
impl IntoValue for bigdecimal::BigDecimal {
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

// Option 类型
impl<T: IntoValue> IntoValue for Option<T> {
    fn into_value(self) -> Value {
        match self {
            Some(v) => v.into_value(),
            None => Value::None,
        }
    }
}
