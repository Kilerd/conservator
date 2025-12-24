//! SQL 参数值类型
//!
//! 提供类型擦除的 `Value` 和 `IntoValue` trait，支持任意实现 `ToSql` 的类型。

use std::error::Error;
use std::fmt::Debug;
use tokio_postgres::types::{private::BytesMut, to_sql_checked, FromSql, IsNull, ToSql, Type};
use uuid::Uuid;

// ============================================================================
// SqlType trait - 用于自定义类型扩展
// ============================================================================

/// 自定义 SQL 类型 trait
///
/// 实现此 trait 来支持自定义 PostgreSQL 类型。
/// 使用 `SqlTypeWrapper<T>` 包装器来获得 `ToSql` 和 `FromSql` 实现。
pub trait SqlType: Sized + Send + Sync + Debug {
    /// 将值序列化为 SQL 参数
    fn to_sql_value(
        &self,
        ty: &Type,
        out: &mut BytesMut,
    ) -> Result<IsNull, Box<dyn Error + Sync + Send>>;

    /// 从数据库结果解析值
    fn from_sql_value(ty: &Type, raw: &[u8]) -> Result<Self, Box<dyn Error + Sync + Send>>;

    /// 从 NULL 值解析（默认返回错误，Option 类型重写此方法）
    fn from_sql_null_value(_ty: &Type) -> Result<Self, Box<dyn Error + Sync + Send>> {
        Err("unexpected NULL value".into())
    }

    /// 检查是否接受此 PostgreSQL 类型
    fn accepts(ty: &Type) -> bool;
}

// ============================================================================
// SqlTypeWrapper - 桥接 SqlType 到 ToSql/FromSql
// ============================================================================

/// SQL 类型包装器，将 `SqlType` 转换为 `ToSql`/`FromSql`
#[derive(Debug, Clone)]
pub struct SqlTypeWrapper<T>(pub T);

impl<T> SqlTypeWrapper<T> {
    pub fn new(value: T) -> Self {
        SqlTypeWrapper(value)
    }

    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T: SqlType> ToSql for SqlTypeWrapper<T> {
    fn to_sql(
        &self,
        ty: &Type,
        out: &mut BytesMut,
    ) -> Result<IsNull, Box<dyn Error + Sync + Send>> {
        self.0.to_sql_value(ty, out)
    }

    fn accepts(ty: &Type) -> bool {
        T::accepts(ty)
    }

    to_sql_checked!();
}

impl<'a, T: SqlType> FromSql<'a> for SqlTypeWrapper<T> {
    fn from_sql(ty: &Type, raw: &'a [u8]) -> Result<Self, Box<dyn Error + Sync + Send>> {
        Ok(SqlTypeWrapper(T::from_sql_value(ty, raw)?))
    }

    fn from_sql_null(ty: &Type) -> Result<Self, Box<dyn Error + Sync + Send>> {
        Ok(SqlTypeWrapper(T::from_sql_null_value(ty)?))
    }

    fn accepts(ty: &Type) -> bool {
        T::accepts(ty)
    }
}

// ============================================================================
// 泛型实现 - Option<T> 和 IntoValue
// ============================================================================

/// 泛型实现：所有 `Option<T: SqlType>` 自动获得 `SqlType`
impl<T: SqlType> SqlType for Option<T> {
    fn to_sql_value(
        &self,
        ty: &Type,
        out: &mut BytesMut,
    ) -> Result<IsNull, Box<dyn Error + Sync + Send>> {
        match self {
            Some(value) => value.to_sql_value(ty, out),
            None => Ok(IsNull::Yes),
        }
    }

    fn from_sql_value(ty: &Type, raw: &[u8]) -> Result<Self, Box<dyn Error + Sync + Send>> {
        Ok(Some(T::from_sql_value(ty, raw)?))
    }

    fn from_sql_null_value(_ty: &Type) -> Result<Self, Box<dyn Error + Sync + Send>> {
        Ok(None)
    }

    fn accepts(ty: &Type) -> bool {
        T::accepts(ty)
    }
}

// ============================================================================
// Value - 类型擦除的 SQL 参数
// ============================================================================

/// 类型擦除的 SQL 参数值
pub struct Value(Box<dyn ToSql + Send + Sync>);

impl Value {
    pub fn new<T: ToSql + Send + Sync + 'static>(v: T) -> Self {
        Value(Box::new(v))
    }

    /// 获取 ToSql 引用，用于 tokio-postgres 参数绑定
    pub fn as_param(&self) -> &(dyn ToSql + Sync) {
        self.0.as_ref()
    }

    pub fn to_tokio_sql_param(
        self,
    ) -> Result<Box<dyn ToSql + Sync + Send + 'static>, crate::Error> {
        Ok(self.0)
    }
}

impl Debug for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Value({:?})", self.0)
    }
}

// ============================================================================
// IntoValue trait - 泛型实现
// ============================================================================

/// 将 Rust 类型转换为 Value 的 trait
pub trait IntoValue {
    fn into_value(self) -> Value;
}

/// 泛型实现：所有 `SqlType` 自动获得 `IntoValue`
impl<T: SqlType + 'static> IntoValue for T {
    fn into_value(self) -> Value {
        Value::new(SqlTypeWrapper(self))
    }
}

// ============================================================================
// 基础类型的 SqlType 实现（简化版宏）
// ============================================================================

/// 为已有 ToSql/FromSql 的类型实现 SqlType（只需一行）
macro_rules! impl_sql_type {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl SqlType for $ty {
                fn to_sql_value(&self, ty: &Type, out: &mut BytesMut) -> Result<IsNull, Box<dyn Error + Sync + Send>> {
                    ToSql::to_sql(self, ty, out)
                }

                fn from_sql_value(ty: &Type, raw: &[u8]) -> Result<Self, Box<dyn Error + Sync + Send>> {
                    FromSql::from_sql(ty, raw)
                }

                fn accepts(ty: &Type) -> bool {
                    <Self as ToSql>::accepts(ty)
                }
            }
        )+
    };
}

// 一次性声明所有基础类型
impl_sql_type!(
    String,
    bool,
    i8,
    i16,
    i32,
    i64,
    u32,
    f32,
    f64,
    Vec<u8>, // PostgreSQL BYTEA
    Uuid,
    chrono::DateTime<chrono::Utc>,
    chrono::DateTime<chrono::Local>,
    chrono::DateTime<chrono::FixedOffset>,
    serde_json::Value,
    rust_decimal::Decimal,
);

#[cfg(test)]
mod test {
    use crate::{Selectable, SqlTypeWrapper};

    #[test]
    fn test_sql_type_with_option() {
        // 验证 Option<T> 自动获得 SqlType
        struct User {
            amount: Option<rust_decimal::Decimal>,
        }

        impl Selectable for User {
            const COLUMN_NAMES: &'static [&'static str] = &["amount"];

            fn from_row(row: &tokio_postgres::Row) -> Result<Self, crate::Error> {
                Ok(Self {
                    amount: {
                        let wrapper: SqlTypeWrapper<_> = row.try_get("amount")?;
                        wrapper.0
                    },
                })
            }
        }
    }
}
