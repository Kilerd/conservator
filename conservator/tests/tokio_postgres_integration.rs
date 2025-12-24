use conservator::{Creatable, Domain, Executor, PooledConnection, Selectable};
use std::sync::atomic::{AtomicU32, Ordering as AtomicOrdering};
use std::sync::OnceLock;
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
    pub age: i32,
    pub score: f64,
    pub is_active: bool,
}

#[derive(Debug, Creatable)]
pub struct CreateUser {
    pub name: String,
    pub email: String,
    pub age: i32,
    pub score: f64,
    pub is_active: bool,
}

#[derive(Debug, Selectable)]
pub struct UserSummary {
    pub id: i32,
    pub name: String,
}

#[derive(Debug, Selectable)]
pub struct UserWithScore {
    pub id: i32,
    pub name: String,
    pub score: f64,
}

// ========== 共享容器设置 ==========

static DOCKER: OnceLock<Cli> = OnceLock::new();
static CONTAINER: OnceLock<Container<'static, Postgres>> = OnceLock::new();
static DB_COUNTER: AtomicU32 = AtomicU32::new(0);

fn get_container() -> &'static Container<'static, Postgres> {
    let docker = DOCKER.get_or_init(Cli::default);
    CONTAINER.get_or_init(|| docker.run(Postgres::default()))
}

/// 为每个测试创建独立的数据库和连接池
async fn setup_test_db() -> PooledConnection {
    let container = get_container();
    let port = container.get_host_port_ipv4(5432);

    // 生成唯一数据库名
    let db_id = DB_COUNTER.fetch_add(1, AtomicOrdering::SeqCst);
    let db_name = format!("test_db_tokio_{}", db_id);

    // 连接到默认 postgres 数据库创建新数据库
    let admin_url = format!("postgres://postgres:postgres@localhost:{}/postgres", port);
    let admin_pool = PooledConnection::from_url(&admin_url).unwrap();
    let admin_client = admin_pool.get().await.unwrap();

    // 创建数据库
    admin_client
        .execute(&format!("CREATE DATABASE {}", db_name), &[])
        .await
        .unwrap();

    drop(admin_client);

    // 连接到新创建的数据库
    let db_url = format!(
        "postgres://postgres:postgres@localhost:{}/{}",
        port, db_name
    );
    let pool = PooledConnection::from_url(&db_url).unwrap();
    let client = pool.get().await.unwrap();

    // 创建测试表
    client
        .execute(
            r#"
        CREATE TABLE users (
            id SERIAL PRIMARY KEY,
            name VARCHAR(255) NOT NULL,
            email VARCHAR(255) NOT NULL,
            age INTEGER NOT NULL,
            score DOUBLE PRECISION NOT NULL,
            is_active BOOLEAN NOT NULL DEFAULT true
        )
    "#,
            &[],
        )
        .await
        .unwrap();

    pool
}

/// 批量插入测试用户（使用 tokio-postgres）
async fn insert_test_users(pool: &PooledConnection) -> Vec<i32> {
    let mut pks = Vec::new();
    let users = vec![
        CreateUser {
            name: "Alice".into(),
            email: "alice@test.com".into(),
            age: 25,
            score: 85.5,
            is_active: true,
        },
        CreateUser {
            name: "Bob".into(),
            email: "bob@test.com".into(),
            age: 30,
            score: 92.0,
            is_active: true,
        },
        CreateUser {
            name: "Charlie".into(),
            email: "charlie@example.com".into(),
            age: 35,
            score: 70.0,
            is_active: false,
        },
    ];

    for user in users {
        let pk = user.insert::<User>().returning_pk(&pool).await.unwrap();
        pks.push(pk);
    }
    pks
}

// ==========================================
// tokio-postgres 连接测试
// ==========================================

#[tokio::test]
async fn test_tokio_postgres_connection() {
    let pool = setup_test_db().await;

    // 测试基本连接
    let client = pool.get().await.unwrap();
    let value: i32 = Executor::query_scalar(&client, "SELECT 1 as test", &[])
        .await
        .unwrap();
    assert_eq!(value, 1);
}

#[tokio::test]
async fn test_select_one_with_connection() {
    let pool = setup_test_db().await;

    // 插入测试数据
    insert_test_users(&pool).await;

    // 使用 SelectBuilder 查询
    let user = User::select()
        .filter(User::COLUMNS.name.eq("Alice".to_string()))
        .one(&pool)
        .await
        .unwrap();

    assert_eq!(user.name, "Alice");
    assert_eq!(user.email, "alice@test.com");
    assert_eq!(user.age, 25);
}

#[tokio::test]
async fn test_select_with_filter() {
    let pool = setup_test_db().await;

    // 插入测试数据
    insert_test_users(&pool).await;

    // 测试过滤查询（使用更具体的条件确保只返回一行）
    let user = User::select()
        .filter(User::COLUMNS.age.eq(30))
        .one(&pool)
        .await
        .unwrap();

    assert_eq!(user.age, 30);
    assert_eq!(user.name, "Bob");
}

#[tokio::test]
async fn test_select_with_multiple_filters() {
    let pool = setup_test_db().await;

    // 插入测试数据
    insert_test_users(&pool).await;

    // 测试多个过滤条件（使用更具体的条件确保只返回一行）
    let user = User::select()
        .filter(User::COLUMNS.is_active.eq(true))
        .filter(User::COLUMNS.age.eq(25))
        .one(&pool)
        .await
        .unwrap();

    assert!(user.is_active);
    assert_eq!(user.age, 25);
    assert_eq!(user.name, "Alice");
}

#[tokio::test]
async fn test_select_returning_projection() {
    let pool = setup_test_db().await;

    // 插入测试数据
    insert_test_users(&pool).await;

    // 测试返回投影类型
    let summary = User::select()
        .returning::<UserSummary>()
        .filter(User::COLUMNS.name.eq("Alice".to_string()))
        .one(&pool)
        .await
        .unwrap();

    assert_eq!(summary.name, "Alice");
    // UserSummary 只有 id 和 name
}

#[tokio::test]
async fn test_select_with_order_by() {
    let pool = setup_test_db().await;

    // 插入测试数据
    insert_test_users(&pool).await;

    // 测试排序查询（先过滤再排序，确保只返回一行）
    let user = User::select()
        .filter(User::COLUMNS.age.eq(35))
        .order_by(User::COLUMNS.age.desc())
        .one(&pool)
        .await
        .unwrap();

    // 应该是年龄最大的
    assert_eq!(user.name, "Charlie");
    assert_eq!(user.age, 35);
}

#[tokio::test]
async fn test_select_with_limit() {
    let pool = setup_test_db().await;

    // 插入测试数据
    insert_test_users(&pool).await;

    // 测试 limit（注意：one_with_connection 只返回一个结果）
    // 这里测试 limit 不影响查询构建
    let user = User::select().limit(1).one(&pool).await.unwrap();

    assert!(!user.name.is_empty());
}

#[tokio::test]
async fn test_select_string_like() {
    let pool = setup_test_db().await;

    // 插入测试数据
    insert_test_users(&pool).await;

    // 测试 LIKE 查询（结合其他条件确保只返回一行）
    let user = User::select()
        .filter(User::COLUMNS.email.like("%@test.com"))
        .filter(User::COLUMNS.name.eq("Alice".to_string()))
        .one(&pool)
        .await
        .unwrap();

    assert!(user.email.ends_with("@test.com"));
    assert_eq!(user.name, "Alice");
}

#[tokio::test]
async fn test_select_numeric_comparison() {
    let pool = setup_test_db().await;

    // 插入测试数据
    insert_test_users(&pool).await;

    // 测试数值比较（使用更具体的条件确保只返回一行）
    let user = User::select()
        .filter(User::COLUMNS.score.eq(92.0))
        .one(&pool)
        .await
        .unwrap();

    assert_eq!(user.score, 92.0);
    assert_eq!(user.name, "Bob");
}

#[tokio::test]
async fn test_select_boolean_filter() {
    let pool = setup_test_db().await;

    // 插入测试数据
    insert_test_users(&pool).await;

    // 测试布尔过滤（结合其他条件确保只返回一行）
    let user = User::select()
        .filter(User::COLUMNS.is_active.eq(true))
        .filter(User::COLUMNS.name.eq("Alice".to_string()))
        .one(&pool)
        .await
        .unwrap();

    assert!(user.is_active);
    assert_eq!(user.name, "Alice");
}

#[tokio::test]
async fn test_select_complex_expression() {
    let pool = setup_test_db().await;

    // 插入测试数据
    insert_test_users(&pool).await;

    // 测试复杂表达式：年龄大于 25 且分数大于 80
    let user = User::select()
        .filter(User::COLUMNS.age.gt(25) & User::COLUMNS.score.gt(80.0))
        .one(&pool)
        .await
        .unwrap();

    assert!(user.age > 25);
    assert!(user.score > 80.0);
}
