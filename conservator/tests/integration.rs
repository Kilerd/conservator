use conservator::{Creatable, Domain, Selectable};
use sqlx::PgPool;
use testcontainers::{clients::Cli, Container};
use testcontainers_modules::postgres::Postgres;

// ========== 测试实体定义 ==========

#[derive(Debug, Domain)]
#[domain(table = "users")]
pub struct User {
    #[domain(primary_key)]
    pub id: i32,
    pub name: String,
    pub email: String,
}

#[derive(Debug, Creatable)]
pub struct CreateUser {
    pub name: String,
    pub email: String,
}

#[derive(Debug, Selectable)]
pub struct UserSummary {
    pub id: i32,
    pub name: String,
}

// ========== 测试设置 ==========

async fn setup_db(docker: &Cli) -> (Container<'_, Postgres>, PgPool) {
    let container = docker.run(Postgres::default());
    let port = container.get_host_port_ipv4(5432);
    let url = format!("postgres://postgres:postgres@localhost:{}/postgres", port);
    
    let pool = PgPool::connect(&url).await.unwrap();
    
    // 创建测试表
    sqlx::query(r#"
        CREATE TABLE users (
            id SERIAL PRIMARY KEY,
            name VARCHAR(255) NOT NULL,
            email VARCHAR(255) NOT NULL
        )
    "#)
    .execute(&pool)
    .await
    .unwrap();
    
    (container, pool)
}

// ========== CRUD 测试 ==========

#[tokio::test]
async fn test_insert_returning_pk() {
    let docker = Cli::default();
    let (_container, pool) = setup_db(&docker).await;
    
    let pk = CreateUser {
        name: "test".into(),
        email: "test@example.com".into(),
    }
    .insert::<User>()
    .returning_pk(&pool)
    .await
    .unwrap();
    
    assert!(pk > 0);
}

#[tokio::test]
async fn test_insert_returning_entity() {
    let docker = Cli::default();
    let (_container, pool) = setup_db(&docker).await;
    
    let user = CreateUser {
        name: "test".into(),
        email: "test@example.com".into(),
    }
    .insert::<User>()
    .returning_entity(&pool)
    .await
    .unwrap();
    
    assert_eq!(user.name, "test");
    assert_eq!(user.email, "test@example.com");
}

#[tokio::test]
async fn test_fetch_by_pk() {
    let docker = Cli::default();
    let (_container, pool) = setup_db(&docker).await;
    
    let pk = CreateUser { name: "find_me".into(), email: "a@b.com".into() }
        .insert::<User>()
        .returning_pk(&pool)
        .await
        .unwrap();
    
    let user = User::fetch_one_by_pk(&pk, &pool).await.unwrap();
    assert_eq!(user.name, "find_me");
    
    let optional = User::find_by_pk(&pk, &pool).await.unwrap();
    assert!(optional.is_some());
}

#[tokio::test]
async fn test_entity_update() {
    let docker = Cli::default();
    let (_container, pool) = setup_db(&docker).await;
    
    let mut user = CreateUser { name: "old".into(), email: "old@test.com".into() }
        .insert::<User>()
        .returning_entity(&pool)
        .await
        .unwrap();
    
    user.name = "new".to_string();
    user.update(&pool).await.unwrap();
    
    let updated = User::fetch_one_by_pk(&user.id, &pool).await.unwrap();
    assert_eq!(updated.name, "new");
}

// ========== SelectBuilder 测试 ==========

#[tokio::test]
async fn test_select_with_filter() {
    let docker = Cli::default();
    let (_container, pool) = setup_db(&docker).await;
    
    // 插入测试数据
    for i in 1..=5 {
        CreateUser { name: format!("user{}", i), email: format!("{}@test.com", i) }
            .insert::<User>()
            .returning_pk(&pool)
            .await
            .unwrap();
    }
    
    let users = User::select()
        .filter(User::COLUMNS.name.like("user%"))
        .all(&pool)
        .await
        .unwrap();
    
    assert_eq!(users.len(), 5);
}

#[tokio::test]
async fn test_select_with_limit_offset() {
    let docker = Cli::default();
    let (_container, pool) = setup_db(&docker).await;
    
    for i in 1..=10 {
        CreateUser { name: format!("user{}", i), email: format!("{}@test.com", i) }
            .insert::<User>()
            .returning_pk(&pool)
            .await
            .unwrap();
    }
    
    let users = User::select()
        .limit(3)
        .offset(2)
        .all(&pool)
        .await
        .unwrap();
    
    assert_eq!(users.len(), 3);
}

#[tokio::test]
async fn test_select_returning_projection() {
    let docker = Cli::default();
    let (_container, pool) = setup_db(&docker).await;
    
    CreateUser { name: "test".into(), email: "test@test.com".into() }
        .insert::<User>()
        .returning_pk(&pool)
        .await
        .unwrap();
    
    let summaries: Vec<UserSummary> = User::select()
        .returning::<UserSummary>()
        .all(&pool)
        .await
        .unwrap();
    
    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0].name, "test");
}

// ========== DeleteBuilder 测试 ==========

#[tokio::test]
async fn test_delete_with_filter() {
    let docker = Cli::default();
    let (_container, pool) = setup_db(&docker).await;
    
    let pk = CreateUser { name: "to_delete".into(), email: "del@test.com".into() }
        .insert::<User>()
        .returning_pk(&pool)
        .await
        .unwrap();
    
    let rows = User::delete()
        .filter(User::COLUMNS.id.eq(pk))
        .execute(&pool)
        .await
        .unwrap();
    
    assert_eq!(rows, 1);
    assert!(User::find_by_pk(&pk, &pool).await.unwrap().is_none());
}

// ========== UpdateBuilder 测试 ==========

#[tokio::test]
async fn test_update_query_builder() {
    let docker = Cli::default();
    let (_container, pool) = setup_db(&docker).await;
    
    let pk = CreateUser { name: "old".into(), email: "old@test.com".into() }
        .insert::<User>()
        .returning_pk(&pool)
        .await
        .unwrap();
    
    let rows = User::update_query()
        .set(User::COLUMNS.name, "new".to_string())
        .set(User::COLUMNS.email, "new@test.com".to_string())
        .filter(User::COLUMNS.id.eq(pk))
        .execute(&pool)
        .await
        .unwrap();
    
    assert_eq!(rows, 1);
    
    let updated = User::fetch_one_by_pk(&pk, &pool).await.unwrap();
    assert_eq!(updated.name, "new");
    assert_eq!(updated.email, "new@test.com");
}

// ========== 批量操作测试 ==========

#[tokio::test]
async fn test_insert_many() {
    let docker = Cli::default();
    let (_container, pool) = setup_db(&docker).await;
    
    let pks = User::insert_many(vec![
        CreateUser { name: "a".into(), email: "a@test.com".into() },
        CreateUser { name: "b".into(), email: "b@test.com".into() },
        CreateUser { name: "c".into(), email: "c@test.com".into() },
    ])
    .returning_pk(&pool)
    .await
    .unwrap();
    
    assert_eq!(pks.len(), 3);
}

