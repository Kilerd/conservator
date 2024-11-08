use async_trait::async_trait;
pub use conservator_macro::{auto, sql, Creatable, Domain};

pub use sqlx::migrate;
pub use sqlx::postgres::PgPoolOptions;
pub use sqlx::FromRow;
pub use sqlx::{Pool, Postgres};

pub type SingleNumberRow = (i32,);

#[derive(FromRow)]
pub struct ExistsRow {
    pub exists: Option<bool>,
}

#[async_trait]
pub trait Domain: Sized {
    const PK_FIELD_NAME: &'static str;
    const TABLE_NAME: &'static str;

    type PrimaryKey;

    async fn find_by_pk<'e, 'c: 'e, E: 'e + sqlx::Executor<'c, Database = sqlx::Postgres>>(
        pk: &Self::PrimaryKey,
        executor: E,
    ) -> Result<Option<Self>, sqlx::Error>;

    async fn fetch_one_by_pk<
        'e,
        'c: 'e,
        E: 'e + ::sqlx::Executor<'c, Database = ::sqlx::Postgres>,
    >(
        pk: &Self::PrimaryKey,
        executor: E,
    ) -> Result<Self, ::sqlx::Error>;

    async fn fetch_all<'e, 'c: 'e, E: 'e + ::sqlx::Executor<'c, Database = ::sqlx::Postgres>>(
        executor: E,
    ) -> Result<Vec<Self>, ::sqlx::Error>;

    async fn create<
        'e,
        'c: 'e,
        E: 'e + ::sqlx::Executor<'c, Database = ::sqlx::Postgres>,
        C: Creatable,
    >(
        data: C,
        executor: E,
    ) -> Result<Self, ::sqlx::Error>;

    async fn delete_by_pk<'e, 'c: 'e, E: 'e + ::sqlx::Executor<'c, Database = ::sqlx::Postgres>>(
        pk: &Self::PrimaryKey,
        executor: E,
    ) -> Result<(), ::sqlx::Error>;

    async fn update<'e, 'c: 'e, E: 'e + ::sqlx::Executor<'c, Database = ::sqlx::Postgres>>(
        entity: Self,
        executor: E,
    ) -> Result<(), ::sqlx::Error>;
}

pub trait Creatable: Send {
    fn get_insert_sql(&self) -> &str;
    fn build<'q, O>(
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
