use async_trait::async_trait;
pub use conservator_macro::{auto, sql, Creatable, Domain, Selectable};

mod field;
mod value;
mod expression;
mod builder;

pub use field::Field;
pub use value::{IntoValue, Value};
pub use expression::{Expression, FieldInfo, Operator, SqlResult};
pub use builder::{DeleteBuilder, InsertBuilder, InsertManyBuilder, IntoOrderedField, JoinType, Order, OrderedField, SelectBuilder, UpdateBuilder};

pub use sqlx::migrate;
pub use sqlx::postgres::PgPoolOptions;
pub use sqlx::FromRow;
pub use sqlx::{Pool, Postgres};

pub type SingleNumberRow = (i32,);

#[derive(FromRow)]
pub struct ExistsRow {
    pub exists: Option<bool>,
}

/// 轻量级 trait，用于 SELECT 返回类型
/// 
/// 实现此 trait 的类型可以作为 `SelectBuilder.returning::<T>()` 的目标类型。
/// `#[derive(Selectable)]` 会自动生成此 trait 和 `sqlx::FromRow` 的实现。
pub trait Selectable: Sized + Send + Unpin + for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> {
    /// 所有列名（用于 SELECT 语句）
    const COLUMN_NAMES: &'static [&'static str];
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

    async fn find_by_pk<'e, 'c: 'e, E: 'e + sqlx::Executor<'c, Database = sqlx::Postgres>>(
        pk: &Self::PrimaryKey,
        executor: E,
    ) -> Result<Option<Self>, sqlx::Error> {
        let pk_field: Field<Self::PrimaryKey> = Field::new(Self::PK_FIELD_NAME, Self::TABLE_NAME, true);
        Self::select().filter(pk_field.eq(*pk)).optional(executor).await
    }

    async fn fetch_one_by_pk<
        'e,
        'c: 'e,
        E: 'e + ::sqlx::Executor<'c, Database = ::sqlx::Postgres>,
    >(
        pk: &Self::PrimaryKey,
        executor: E,
    ) -> Result<Self, ::sqlx::Error> {
        let pk_field: Field<Self::PrimaryKey> = Field::new(Self::PK_FIELD_NAME, Self::TABLE_NAME, true);
        Self::select().filter(pk_field.eq(*pk)).one(executor).await
    }

    async fn fetch_all<'e, 'c: 'e, E: 'e + ::sqlx::Executor<'c, Database = ::sqlx::Postgres>>(
        executor: E,
    ) -> Result<Vec<Self>, ::sqlx::Error> {
        Self::select().all(executor).await
    }

    async fn delete_by_pk<'e, 'c: 'e, E: 'e + ::sqlx::Executor<'c, Database = ::sqlx::Postgres>>(
        pk: &Self::PrimaryKey,
        executor: E,
    ) -> Result<u64, ::sqlx::Error> {
        let pk_field: Field<Self::PrimaryKey> = Field::new(Self::PK_FIELD_NAME, Self::TABLE_NAME, true);
        DeleteBuilder::<Self>::new().filter(pk_field.eq(*pk)).execute(executor).await
    }

    /// 更新实体到数据库
    /// 
    /// 此方法由 `#[derive(Domain)]` 宏生成具体实现
    async fn update<'e, 'c: 'e, E: 'e + ::sqlx::Executor<'c, Database = ::sqlx::Postgres>>(
        &self,
        executor: E,
    ) -> Result<(), ::sqlx::Error>;
}

pub trait Creatable: Send + Sized {
    fn get_columns(&self) -> &str;
    fn get_insert_sql(&self) -> &str;
    fn get_batch_insert_sql(&self, idx: usize) -> String;
    fn build_for_query_as<'q, O>(
        self,
        e: ::sqlx::query::QueryAs<
            'q,
            ::sqlx::Postgres,
            O,
            <::sqlx::Postgres as ::sqlx::database::HasArguments<'q>>::Arguments,
        >,
    ) -> ::sqlx::query::QueryAs<
        'q,
        ::sqlx::Postgres,
        O,
        <::sqlx::Postgres as ::sqlx::database::HasArguments<'q>>::Arguments,
    >;
    fn build_for_query<'q>(
        self,
        e: ::sqlx::query::Query<
            'q,
            ::sqlx::Postgres,
            <::sqlx::Postgres as ::sqlx::database::HasArguments<'q>>::Arguments,
        >,
    ) -> ::sqlx::query::Query<
        'q,
        ::sqlx::Postgres,
        <::sqlx::Postgres as ::sqlx::database::HasArguments<'q>>::Arguments,
    >;
    fn bind_to_query_scalar<'q, O>(
        self,
        e: ::sqlx::query::QueryScalar<
            'q,
            ::sqlx::Postgres,
            O,
            <::sqlx::Postgres as ::sqlx::database::HasArguments<'q>>::Arguments,
        >,
    ) -> ::sqlx::query::QueryScalar<
        'q,
        ::sqlx::Postgres,
        O,
        <::sqlx::Postgres as ::sqlx::database::HasArguments<'q>>::Arguments,
    >;

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


