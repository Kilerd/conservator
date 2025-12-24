use async_trait::async_trait;
pub use conservator_macro::{sql, Creatable, Domain, Selectable};

mod builder;
mod conn;
mod error;
mod executor;
mod expression;
mod field;
mod value;

pub use builder::{
    DeleteBuilder, InsertBuilder, InsertManyBuilder, IntoOrderedField, JoinType, Order,
    OrderedField, SelectBuilder, UpdateBuilder,
};
pub use conn::{Connection, PooledConnection, Transaction};
pub use error::Error;
pub use executor::Executor;
pub use expression::{Expression, FieldInfo, Operator, SqlResult};
pub use field::Field;
pub use value::{IntoValue, SqlType, SqlTypeWrapper, Value};

#[cfg(feature = "migrate")]
pub use sqlx::migrate;
#[cfg(feature = "migrate")]
pub use sqlx::postgres::PgPoolOptions;
#[cfg(feature = "migrate")]
pub use sqlx::{Pool, Postgres};

pub type SingleNumberRow = (i32,);

/// 轻量级 trait，用于 SELECT 返回类型
///
/// 实现此 trait 的类型可以作为 `SelectBuilder.returning::<T>()` 的目标类型。
/// `#[derive(Selectable)]` 会自动生成此 trait 的实现。
pub trait Selectable: Sized + Send + Unpin {
    /// 所有列名（用于 SELECT 语句）
    const COLUMN_NAMES: &'static [&'static str];

    /// 从 `tokio_postgres::Row` 创建实例
    ///
    /// 这是 `Selectable` 的核心方法，用于将数据库行转换为 Rust 类型。
    fn from_row(row: &tokio_postgres::Row) -> Result<Self, Error>;
}

#[async_trait]
pub trait Domain: Selectable {
    const PK_FIELD_NAME: &'static str;
    const TABLE_NAME: &'static str;

    type PrimaryKey: IntoValue + Copy + Send + Sync;

    fn select() -> SelectBuilder<Self, Self> {
        SelectBuilder::<Self, Self>::new()
    }

    fn delete() -> DeleteBuilder<Self> {
        DeleteBuilder::<Self>::new()
    }

    fn update_query() -> UpdateBuilder<Self> {
        UpdateBuilder::<Self>::new()
    }

    fn insert<C: Creatable>(data: C) -> InsertBuilder<Self, C> {
        InsertBuilder::new(data)
    }
    fn insert_many<C: Creatable>(data: Vec<C>) -> InsertManyBuilder<Self, C> {
        InsertManyBuilder::new(data)
    }

    async fn find_by_pk<E: Executor>(
        pk: &Self::PrimaryKey,
        executor: &E,
    ) -> Result<Option<Self>, Error> {
        let pk_field: Field<Self::PrimaryKey> =
            Field::new(Self::PK_FIELD_NAME, Self::TABLE_NAME, true);
        Self::select()
            .filter(pk_field.eq(*pk))
            .optional(executor)
            .await
    }

    async fn fetch_one_by_pk<E: Executor>(
        pk: &Self::PrimaryKey,
        executor: &E,
    ) -> Result<Self, Error> {
        let pk_field: Field<Self::PrimaryKey> =
            Field::new(Self::PK_FIELD_NAME, Self::TABLE_NAME, true);
        Self::select().filter(pk_field.eq(*pk)).one(executor).await
    }

    async fn fetch_all<E: Executor>(executor: &E) -> Result<Vec<Self>, Error> {
        Self::select().all(executor).await
    }

    async fn delete_by_pk<E: Executor>(pk: &Self::PrimaryKey, executor: &E) -> Result<u64, Error> {
        let pk_field: Field<Self::PrimaryKey> =
            Field::new(Self::PK_FIELD_NAME, Self::TABLE_NAME, true);
        DeleteBuilder::<Self>::new()
            .filter(pk_field.eq(*pk))
            .execute(executor)
            .await
    }

    /// 更新实体到数据库
    ///
    /// 此方法由 `#[derive(Domain)]` 宏生成具体实现
    async fn update<E: Executor>(&self, executor: &E) -> Result<(), Error>;
}

pub trait Creatable: Send + Sized {
    fn get_columns(&self) -> &str;
    fn get_insert_sql(&self) -> &str;
    fn get_batch_insert_sql(&self, idx: usize) -> String;

    /// 获取参数值列表（用于 tokio-postgres）
    fn get_values(&self) -> Vec<Value>;

    /// 获取批量插入的参数值列表（用于 tokio-postgres）
    fn get_batch_values(&self, idx: usize) -> Vec<Value>;

    /// 创建 InsertBuilder 用于插入数据
    fn insert<T: Domain>(self) -> InsertBuilder<T, Self> {
        InsertBuilder::new(self)
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn compile_fail() {
        let t = trybuild::TestCases::new();
        t.compile_fail("tests/fail/*.rs");
    }

    #[test]
    fn compile_pass() {
        let t = trybuild::TestCases::new();
        t.pass("tests/pass/*.rs");
    }
}
