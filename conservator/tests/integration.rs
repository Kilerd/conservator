use conservator::{Creatable, Domain, Selectable};
use sqlx::PgPool;
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

// 带 nullable 字段的实体（用于 IS NULL 测试）
#[derive(Debug, Domain)]
#[domain(table = "profiles")]
pub struct Profile {
    #[domain(primary_key)]
    pub id: i32,
    pub user_id: i32,
    pub bio: Option<String>,
    pub website: Option<String>,
}

#[derive(Debug, Creatable)]
pub struct CreateProfile {
    pub user_id: i32,
    pub bio: Option<String>,
    pub website: Option<String>,
}

// 多数据类型实体
#[derive(Debug, Domain)]
#[domain(table = "products")]
pub struct Product {
    #[domain(primary_key)]
    pub id: i32,
    pub uuid: uuid::Uuid,
    pub name: String,
    pub price: sqlx::types::BigDecimal,
    pub metadata: serde_json::Value,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Creatable)]
pub struct CreateProduct {
    pub uuid: uuid::Uuid,
    pub name: String,
    pub price: sqlx::types::BigDecimal,
    pub metadata: serde_json::Value,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

// ========== 共享容器设置 ==========

static DOCKER: OnceLock<Cli> = OnceLock::new();
static CONTAINER: OnceLock<Container<'static, Postgres>> = OnceLock::new();
static DB_COUNTER: AtomicU32 = AtomicU32::new(0);

fn get_container() -> &'static Container<'static, Postgres> {
    let docker = DOCKER.get_or_init(Cli::default);
    CONTAINER.get_or_init(|| docker.run(Postgres::default()))
}

/// 为每个测试创建独立的数据库
async fn setup_test_db() -> PgPool {
    let container = get_container();
    let port = container.get_host_port_ipv4(5432);

    // 生成唯一数据库名
    let db_id = DB_COUNTER.fetch_add(1, AtomicOrdering::SeqCst);
    let db_name = format!("test_db_{}", db_id);

    // 连接到默认 postgres 数据库创建新数据库
    let admin_url = format!("postgres://postgres:postgres@localhost:{}/postgres", port);
    let admin_pool = PgPool::connect(&admin_url).await.unwrap();

    sqlx::query(&format!("CREATE DATABASE {}", db_name))
        .execute(&admin_pool)
        .await
        .unwrap();

    admin_pool.close().await;

    // 连接到新创建的数据库
    let db_url = format!(
        "postgres://postgres:postgres@localhost:{}/{}",
        port, db_name
    );
    let pool = PgPool::connect(&db_url).await.unwrap();

    // 创建测试表
    sqlx::query(
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
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        r#"
        CREATE TABLE profiles (
            id SERIAL PRIMARY KEY,
            user_id INTEGER NOT NULL,
            bio TEXT,
            website VARCHAR(255)
        )
    "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    sqlx::query(
        r#"
        CREATE TABLE products (
            id SERIAL PRIMARY KEY,
            uuid UUID NOT NULL,
            name VARCHAR(255) NOT NULL,
            price NUMERIC(10, 2) NOT NULL,
            metadata JSONB NOT NULL,
            created_at TIMESTAMPTZ NOT NULL
        )
    "#,
    )
    .execute(&pool)
    .await
    .unwrap();

    pool
}

/// 批量插入测试用户
async fn insert_test_users(pool: &PgPool) -> Vec<i32> {
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
        CreateUser {
            name: "Diana".into(),
            email: "diana@test.com".into(),
            age: 22,
            score: 78.5,
            is_active: true,
        },
        CreateUser {
            name: "Eve".into(),
            email: "eve@example.com".into(),
            age: 28,
            score: 95.0,
            is_active: true,
        },
    ];

    for user in users {
        let pk = user.insert::<User>().returning_pk(pool).await.unwrap();
        pks.push(pk);
    }
    pks
}

// ==========================================
// CRUD 基础测试
// ==========================================

#[tokio::test]
async fn test_insert_returning_pk() {
    let pool = setup_test_db().await;

    let pk = CreateUser {
        name: "test".into(),
        email: "test@example.com".into(),
        age: 25,
        score: 80.0,
        is_active: true,
    }
    .insert::<User>()
    .returning_pk(&pool)
    .await
    .unwrap();

    assert!(pk > 0);
}

#[tokio::test]
async fn test_insert_returning_entity() {
    let pool = setup_test_db().await;

    let user = CreateUser {
        name: "test".into(),
        email: "test@example.com".into(),
        age: 30,
        score: 88.0,
        is_active: false,
    }
    .insert::<User>()
    .returning_entity(&pool)
    .await
    .unwrap();

    assert_eq!(user.name, "test");
    assert_eq!(user.email, "test@example.com");
    assert_eq!(user.age, 30);
    assert!(!user.is_active);
}

#[tokio::test]
async fn test_fetch_by_pk() {
    let pool = setup_test_db().await;

    let pk = CreateUser {
        name: "find_me".into(),
        email: "a@b.com".into(),
        age: 20,
        score: 100.0,
        is_active: true,
    }
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
    let pool = setup_test_db().await;

    let mut user = CreateUser {
        name: "old".into(),
        email: "old@test.com".into(),
        age: 20,
        score: 50.0,
        is_active: true,
    }
    .insert::<User>()
    .returning_entity(&pool)
    .await
    .unwrap();

    user.name = "new".to_string();
    user.age = 21;
    user.update(&pool).await.unwrap();

    let updated = User::fetch_one_by_pk(&user.id, &pool).await.unwrap();
    assert_eq!(updated.name, "new");
    assert_eq!(updated.age, 21);
}

// ==========================================
// 表达式操作符测试
// ==========================================

#[tokio::test]
async fn test_eq_operator() {
    let pool = setup_test_db().await;
    insert_test_users(&pool).await;

    let users = User::select()
        .filter(User::COLUMNS.name.eq("Alice".to_string()))
        .all(&pool)
        .await
        .unwrap();

    assert_eq!(users.len(), 1);
    assert_eq!(users[0].name, "Alice");
}

#[tokio::test]
async fn test_ne_operator() {
    let pool = setup_test_db().await;
    insert_test_users(&pool).await;

    let users = User::select()
        .filter(User::COLUMNS.name.ne("Alice".to_string()))
        .all(&pool)
        .await
        .unwrap();

    assert_eq!(users.len(), 4);
    assert!(users.iter().all(|u| u.name != "Alice"));
}

#[tokio::test]
async fn test_gt_operator() {
    let pool = setup_test_db().await;
    insert_test_users(&pool).await;

    let users = User::select()
        .filter(User::COLUMNS.age.gt(28))
        .all(&pool)
        .await
        .unwrap();

    // Bob(30) and Charlie(35)
    assert_eq!(users.len(), 2);
    assert!(users.iter().all(|u| u.age > 28));
}

#[tokio::test]
async fn test_lt_operator() {
    let pool = setup_test_db().await;
    insert_test_users(&pool).await;

    let users = User::select()
        .filter(User::COLUMNS.age.lt(25))
        .all(&pool)
        .await
        .unwrap();

    // Diana(22)
    assert_eq!(users.len(), 1);
    assert_eq!(users[0].name, "Diana");
}

#[tokio::test]
async fn test_gte_operator() {
    let pool = setup_test_db().await;
    insert_test_users(&pool).await;

    let users = User::select()
        .filter(User::COLUMNS.age.gte(30))
        .all(&pool)
        .await
        .unwrap();

    // Bob(30) and Charlie(35)
    assert_eq!(users.len(), 2);
}

#[tokio::test]
async fn test_lte_operator() {
    let pool = setup_test_db().await;
    insert_test_users(&pool).await;

    let users = User::select()
        .filter(User::COLUMNS.age.lte(25))
        .all(&pool)
        .await
        .unwrap();

    // Alice(25), Diana(22)
    assert_eq!(users.len(), 2);
}

#[tokio::test]
async fn test_between_operator() {
    let pool = setup_test_db().await;
    insert_test_users(&pool).await;

    let users = User::select()
        .filter(User::COLUMNS.age.between(26, 32))
        .all(&pool)
        .await
        .unwrap();

    // Bob(30), Eve(28)
    assert_eq!(users.len(), 2);
    assert!(users.iter().all(|u| u.age >= 26 && u.age <= 32));
}

#[tokio::test]
async fn test_in_list_operator() {
    let pool = setup_test_db().await;
    insert_test_users(&pool).await;

    let users = User::select()
        .filter(User::COLUMNS.name.in_list(vec![
            "Alice".to_string(),
            "Bob".to_string(),
            "Eve".to_string(),
        ]))
        .all(&pool)
        .await
        .unwrap();

    assert_eq!(users.len(), 3);
    let names: Vec<_> = users.iter().map(|u| u.name.as_str()).collect();
    assert!(names.contains(&"Alice"));
    assert!(names.contains(&"Bob"));
    assert!(names.contains(&"Eve"));
}

#[tokio::test]
async fn test_in_list_with_integers() {
    let pool = setup_test_db().await;
    insert_test_users(&pool).await;

    let users = User::select()
        .filter(User::COLUMNS.age.in_list(vec![25, 28, 35]))
        .all(&pool)
        .await
        .unwrap();

    // Alice(25), Eve(28), Charlie(35)
    assert_eq!(users.len(), 3);
}

#[tokio::test]
async fn test_is_null_operator() {
    let pool = setup_test_db().await;

    // Insert profiles with nullable fields
    CreateProfile {
        user_id: 1,
        bio: Some("Hello".into()),
        website: None,
    }
    .insert::<Profile>()
    .returning_pk(&pool)
    .await
    .unwrap();

    CreateProfile {
        user_id: 2,
        bio: None,
        website: Some("https://example.com".into()),
    }
    .insert::<Profile>()
    .returning_pk(&pool)
    .await
    .unwrap();

    let profiles = Profile::select()
        .filter(Profile::COLUMNS.bio.is_null())
        .all(&pool)
        .await
        .unwrap();

    assert_eq!(profiles.len(), 1);
    assert_eq!(profiles[0].user_id, 2);
}

#[tokio::test]
async fn test_is_not_null_operator() {
    let pool = setup_test_db().await;

    CreateProfile {
        user_id: 1,
        bio: Some("Hello".into()),
        website: None,
    }
    .insert::<Profile>()
    .returning_pk(&pool)
    .await
    .unwrap();

    CreateProfile {
        user_id: 2,
        bio: None,
        website: Some("https://example.com".into()),
    }
    .insert::<Profile>()
    .returning_pk(&pool)
    .await
    .unwrap();

    let profiles = Profile::select()
        .filter(Profile::COLUMNS.bio.is_not_null())
        .all(&pool)
        .await
        .unwrap();

    assert_eq!(profiles.len(), 1);
    assert!(profiles[0].bio.is_some());
}

#[tokio::test]
async fn test_like_operator() {
    let pool = setup_test_db().await;
    insert_test_users(&pool).await;

    // Users with @test.com email
    let users = User::select()
        .filter(User::COLUMNS.email.like("%@test.com"))
        .all(&pool)
        .await
        .unwrap();

    assert_eq!(users.len(), 3); // Alice, Bob, Diana
}

#[tokio::test]
async fn test_like_operator_prefix() {
    let pool = setup_test_db().await;
    insert_test_users(&pool).await;

    let users = User::select()
        .filter(User::COLUMNS.name.like("A%"))
        .all(&pool)
        .await
        .unwrap();

    assert_eq!(users.len(), 1);
    assert_eq!(users[0].name, "Alice");
}

// ==========================================
// 复杂表达式组合测试
// ==========================================

#[tokio::test]
async fn test_and_combination() {
    let pool = setup_test_db().await;
    insert_test_users(&pool).await;

    let users = User::select()
        .filter(User::COLUMNS.is_active.eq(true).and(User::COLUMNS.age.gt(25)))
        .all(&pool)
        .await
        .unwrap();

    // Bob(30), Eve(28) - active and age > 25
    assert_eq!(users.len(), 2);
    assert!(users.iter().all(|u| u.is_active && u.age > 25));
}

#[tokio::test]
async fn test_or_combination() {
    let pool = setup_test_db().await;
    insert_test_users(&pool).await;

    let users = User::select()
        .filter(
            User::COLUMNS
                .name
                .eq("Alice".to_string())
                .or(User::COLUMNS.name.eq("Bob".to_string())),
        )
        .all(&pool)
        .await
        .unwrap();

    assert_eq!(users.len(), 2);
}

#[tokio::test]
async fn test_bitand_operator() {
    let pool = setup_test_db().await;
    insert_test_users(&pool).await;

    // Using & operator instead of .and()
    let users = User::select()
        .filter(User::COLUMNS.is_active.eq(true) & User::COLUMNS.score.gt(90.0))
        .all(&pool)
        .await
        .unwrap();

    // Bob(92.0), Eve(95.0)
    assert_eq!(users.len(), 2);
}

#[tokio::test]
async fn test_bitor_operator() {
    let pool = setup_test_db().await;
    insert_test_users(&pool).await;

    // Using | operator instead of .or()
    let users = User::select()
        .filter(User::COLUMNS.age.lt(23) | User::COLUMNS.age.gt(32))
        .all(&pool)
        .await
        .unwrap();

    // Diana(22), Charlie(35)
    assert_eq!(users.len(), 2);
}

#[tokio::test]
async fn test_nested_expressions() {
    let pool = setup_test_db().await;
    insert_test_users(&pool).await;

    // (is_active = true AND age > 25) OR (score > 90)
    let users = User::select()
        .filter(
            (User::COLUMNS.is_active.eq(true) & User::COLUMNS.age.gt(25))
                | User::COLUMNS.score.gt(90.0),
        )
        .all(&pool)
        .await
        .unwrap();

    // Bob(active, 30, 92), Eve(active, 28, 95) match first condition
    // Eve also matches score > 90
    assert!(users.len() >= 2);
}

#[tokio::test]
async fn test_multiple_filter_calls() {
    let pool = setup_test_db().await;
    insert_test_users(&pool).await;

    // Multiple .filter() calls should AND together
    let users = User::select()
        .filter(User::COLUMNS.is_active.eq(true))
        .filter(User::COLUMNS.age.gt(25))
        .filter(User::COLUMNS.score.gt(80.0))
        .all(&pool)
        .await
        .unwrap();

    // Bob(30, 92.0), Eve(28, 95.0)
    assert_eq!(users.len(), 2);
}

// ==========================================
// 排序测试
// ==========================================

#[tokio::test]
async fn test_order_by_asc() {
    let pool = setup_test_db().await;
    insert_test_users(&pool).await;

    let users = User::select()
        .order_by(User::COLUMNS.age)
        .all(&pool)
        .await
        .unwrap();

    let ages: Vec<i32> = users.iter().map(|u| u.age).collect();
    assert_eq!(ages, vec![22, 25, 28, 30, 35]);
}

#[tokio::test]
async fn test_order_by_desc() {
    let pool = setup_test_db().await;
    insert_test_users(&pool).await;

    let users = User::select()
        .order_by(User::COLUMNS.age.desc())
        .all(&pool)
        .await
        .unwrap();

    let ages: Vec<i32> = users.iter().map(|u| u.age).collect();
    assert_eq!(ages, vec![35, 30, 28, 25, 22]);
}

#[tokio::test]
async fn test_order_by_multiple_fields() {
    let pool = setup_test_db().await;
    insert_test_users(&pool).await;

    let users = User::select()
        .order_by(User::COLUMNS.is_active.desc()) // active first
        .order_by(User::COLUMNS.name) // then by name
        .all(&pool)
        .await
        .unwrap();

    // Active users first (alphabetically): Alice, Bob, Diana, Eve
    // Then inactive: Charlie
    assert_eq!(users[0].name, "Alice");
    assert_eq!(users[users.len() - 1].name, "Charlie");
}

#[tokio::test]
async fn test_order_by_three_fields() {
    let pool = setup_test_db().await;

    // 插入更多测试数据以便多重排序
    let users_data = vec![
        CreateUser { name: "Alice".into(), email: "a1@test.com".into(), age: 25, score: 80.0, is_active: true },
        CreateUser { name: "Alice".into(), email: "a2@test.com".into(), age: 30, score: 90.0, is_active: true },
        CreateUser { name: "Bob".into(), email: "b1@test.com".into(), age: 25, score: 85.0, is_active: true },
        CreateUser { name: "Bob".into(), email: "b2@test.com".into(), age: 25, score: 75.0, is_active: false },
        CreateUser { name: "Charlie".into(), email: "c1@test.com".into(), age: 30, score: 70.0, is_active: true },
    ];

    for user in users_data {
        user.insert::<User>().returning_pk(&pool).await.unwrap();
    }

    // 三重排序: name ASC -> age DESC -> score ASC
    let users = User::select()
        .order_by(User::COLUMNS.name)
        .order_by(User::COLUMNS.age.desc())
        .order_by(User::COLUMNS.score)
        .all(&pool)
        .await
        .unwrap();

    // Alice: age 30 first (desc), then age 25
    assert_eq!(users[0].name, "Alice");
    assert_eq!(users[0].age, 30);
    assert_eq!(users[1].name, "Alice");
    assert_eq!(users[1].age, 25);

    // Bob: both age 25, score 75 first (asc), then 85
    assert_eq!(users[2].name, "Bob");
    assert_eq!(users[2].score, 75.0);
    assert_eq!(users[3].name, "Bob");
    assert_eq!(users[3].score, 85.0);

    // Charlie last
    assert_eq!(users[4].name, "Charlie");
}

#[tokio::test]
async fn test_order_by_mixed_asc_desc() {
    let pool = setup_test_db().await;
    insert_test_users(&pool).await;

    // score DESC, age ASC
    let users = User::select()
        .order_by(User::COLUMNS.score.desc())
        .order_by(User::COLUMNS.age)
        .all(&pool)
        .await
        .unwrap();

    // First by score descending: Eve(95), Bob(92), Alice(85.5), Diana(78.5), Charlie(70)
    assert_eq!(users[0].name, "Eve");
    assert_eq!(users[0].score, 95.0);
    assert_eq!(users[4].name, "Charlie");
    assert_eq!(users[4].score, 70.0);
}

#[tokio::test]
async fn test_order_by_with_same_values() {
    let pool = setup_test_db().await;

    // 插入具有相同值的数据
    let users_data = vec![
        CreateUser { name: "User A".into(), email: "a@test.com".into(), age: 25, score: 80.0, is_active: true },
        CreateUser { name: "User B".into(), email: "b@test.com".into(), age: 25, score: 80.0, is_active: true },
        CreateUser { name: "User C".into(), email: "c@test.com".into(), age: 25, score: 80.0, is_active: true },
    ];

    for user in users_data {
        user.insert::<User>().returning_pk(&pool).await.unwrap();
    }

    // 相同 age 和 score，按 name 排序
    let users = User::select()
        .order_by(User::COLUMNS.age)
        .order_by(User::COLUMNS.score)
        .order_by(User::COLUMNS.name)
        .all(&pool)
        .await
        .unwrap();

    assert_eq!(users[0].name, "User A");
    assert_eq!(users[1].name, "User B");
    assert_eq!(users[2].name, "User C");
}

#[tokio::test]
async fn test_order_by_with_filter_and_limit() {
    let pool = setup_test_db().await;
    insert_test_users(&pool).await;

    // 过滤 + 多重排序 + 分页
    let users = User::select()
        .filter(User::COLUMNS.is_active.eq(true))
        .order_by(User::COLUMNS.score.desc())
        .order_by(User::COLUMNS.age)
        .limit(2)
        .all(&pool)
        .await
        .unwrap();

    // Active users by score desc: Eve(95), Bob(92), Alice(85.5), Diana(78.5)
    // Top 2: Eve, Bob
    assert_eq!(users.len(), 2);
    assert_eq!(users[0].name, "Eve");
    assert_eq!(users[1].name, "Bob");
}

#[tokio::test]
async fn test_order_by_with_limit() {
    let pool = setup_test_db().await;
    insert_test_users(&pool).await;

    // Top 3 users by score
    let users = User::select()
        .order_by(User::COLUMNS.score.desc())
        .limit(3)
        .all(&pool)
        .await
        .unwrap();

    assert_eq!(users.len(), 3);
    assert_eq!(users[0].name, "Eve"); // 95.0
    assert_eq!(users[1].name, "Bob"); // 92.0
    assert_eq!(users[2].name, "Alice"); // 85.5
}

// ==========================================
// 分页测试
// ==========================================

#[tokio::test]
async fn test_limit_offset_pagination() {
    let pool = setup_test_db().await;
    insert_test_users(&pool).await;

    // Page 1 (first 2)
    let page1 = User::select()
        .order_by(User::COLUMNS.id)
        .limit(2)
        .offset(0)
        .all(&pool)
        .await
        .unwrap();

    // Page 2 (next 2)
    let page2 = User::select()
        .order_by(User::COLUMNS.id)
        .limit(2)
        .offset(2)
        .all(&pool)
        .await
        .unwrap();

    // Page 3 (last 1)
    let page3 = User::select()
        .order_by(User::COLUMNS.id)
        .limit(2)
        .offset(4)
        .all(&pool)
        .await
        .unwrap();

    assert_eq!(page1.len(), 2);
    assert_eq!(page2.len(), 2);
    assert_eq!(page3.len(), 1);

    // Ensure no overlap
    let all_ids: Vec<i32> = page1
        .iter()
        .chain(page2.iter())
        .chain(page3.iter())
        .map(|u| u.id)
        .collect();
    assert_eq!(all_ids.len(), 5);
}

// ==========================================
// 边界条件测试
// ==========================================

#[tokio::test]
async fn test_empty_result() {
    let pool = setup_test_db().await;
    insert_test_users(&pool).await;

    let users = User::select()
        .filter(User::COLUMNS.name.eq("NonExistent".to_string()))
        .all(&pool)
        .await
        .unwrap();

    assert!(users.is_empty());
}

#[tokio::test]
async fn test_optional_not_found() {
    let pool = setup_test_db().await;

    let result = User::find_by_pk(&99999, &pool).await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_one_not_found_error() {
    let pool = setup_test_db().await;

    let result = User::select()
        .filter(User::COLUMNS.name.eq("NonExistent".to_string()))
        .one(&pool)
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_optional_found() {
    let pool = setup_test_db().await;
    insert_test_users(&pool).await;

    let result = User::select()
        .filter(User::COLUMNS.name.eq("Alice".to_string()))
        .optional(&pool)
        .await
        .unwrap();

    assert!(result.is_some());
    assert_eq!(result.unwrap().name, "Alice");
}

#[tokio::test]
async fn test_optional_not_found_returns_none() {
    let pool = setup_test_db().await;

    let result = User::select()
        .filter(User::COLUMNS.name.eq("NonExistent".to_string()))
        .optional(&pool)
        .await
        .unwrap();

    assert!(result.is_none());
}

#[tokio::test]
async fn test_special_characters_in_string() {
    let pool = setup_test_db().await;

    let pk = CreateUser {
        name: "O'Brien".into(),
        email: "o'brien@test.com".into(),
        age: 40,
        score: 75.0,
        is_active: true,
    }
    .insert::<User>()
    .returning_pk(&pool)
    .await
    .unwrap();

    let user = User::fetch_one_by_pk(&pk, &pool).await.unwrap();
    assert_eq!(user.name, "O'Brien");
}

#[tokio::test]
async fn test_unicode_characters() {
    let pool = setup_test_db().await;

    let pk = CreateUser {
        name: "张三".into(),
        email: "zhangsan@test.com".into(),
        age: 25,
        score: 88.0,
        is_active: true,
    }
    .insert::<User>()
    .returning_pk(&pool)
    .await
    .unwrap();

    let user = User::fetch_one_by_pk(&pk, &pool).await.unwrap();
    assert_eq!(user.name, "张三");
}

#[tokio::test]
async fn test_empty_string() {
    let pool = setup_test_db().await;

    let pk = CreateUser {
        name: "".into(),
        email: "empty@test.com".into(),
        age: 0,
        score: 0.0,
        is_active: true,
    }
    .insert::<User>()
    .returning_pk(&pool)
    .await
    .unwrap();

    let user = User::fetch_one_by_pk(&pk, &pool).await.unwrap();
    assert_eq!(user.name, "");
}

// ==========================================
// 多数据类型测试
// ==========================================

#[tokio::test]
async fn test_uuid_type() {
    let pool = setup_test_db().await;

    let uuid = uuid::Uuid::new_v4();
    let pk = CreateProduct {
        uuid,
        name: "Widget".into(),
        price: "99.99".parse().unwrap(),
        metadata: serde_json::json!({"category": "electronics"}),
        created_at: chrono::Utc::now(),
    }
    .insert::<Product>()
    .returning_pk(&pool)
    .await
    .unwrap();

    let product = Product::fetch_one_by_pk(&pk, &pool).await.unwrap();
    assert_eq!(product.uuid, uuid);
}

#[tokio::test]
async fn test_bigdecimal_type() {
    let pool = setup_test_db().await;

    let price: sqlx::types::BigDecimal = "1234.56".parse().unwrap();
    let pk = CreateProduct {
        uuid: uuid::Uuid::new_v4(),
        name: "Expensive Item".into(),
        price: price.clone(),
        metadata: serde_json::json!({}),
        created_at: chrono::Utc::now(),
    }
    .insert::<Product>()
    .returning_pk(&pool)
    .await
    .unwrap();

    let product = Product::fetch_one_by_pk(&pk, &pool).await.unwrap();
    assert_eq!(product.price, price);
}

#[tokio::test]
async fn test_json_type() {
    let pool = setup_test_db().await;

    let metadata = serde_json::json!({
        "tags": ["sale", "featured"],
        "dimensions": {
            "width": 10,
            "height": 20
        }
    });

    let pk = CreateProduct {
        uuid: uuid::Uuid::new_v4(),
        name: "JSON Test".into(),
        price: "50.00".parse().unwrap(),
        metadata: metadata.clone(),
        created_at: chrono::Utc::now(),
    }
    .insert::<Product>()
    .returning_pk(&pool)
    .await
    .unwrap();

    let product = Product::fetch_one_by_pk(&pk, &pool).await.unwrap();
    assert_eq!(product.metadata, metadata);
}

#[tokio::test]
async fn test_datetime_type() {
    let pool = setup_test_db().await;

    let created_at = chrono::Utc::now();
    let pk = CreateProduct {
        uuid: uuid::Uuid::new_v4(),
        name: "DateTime Test".into(),
        price: "10.00".parse().unwrap(),
        metadata: serde_json::json!({}),
        created_at,
    }
    .insert::<Product>()
    .returning_pk(&pool)
    .await
    .unwrap();

    let product = Product::fetch_one_by_pk(&pk, &pool).await.unwrap();
    // Compare with some tolerance for database precision
    let diff = (product.created_at - created_at).num_milliseconds().abs();
    assert!(diff < 1000); // Within 1 second
}

// ==========================================
// Projection 测试
// ==========================================

#[tokio::test]
async fn test_returning_projection() {
    let pool = setup_test_db().await;
    insert_test_users(&pool).await;

    let summaries: Vec<UserSummary> = User::select()
        .returning::<UserSummary>()
        .all(&pool)
        .await
        .unwrap();

    assert_eq!(summaries.len(), 5);
    // UserSummary only has id and name
    assert!(!summaries[0].name.is_empty());
}

#[tokio::test]
async fn test_returning_with_filter() {
    let pool = setup_test_db().await;
    insert_test_users(&pool).await;

    let summaries: Vec<UserWithScore> = User::select()
        .filter(User::COLUMNS.score.gt(80.0))
        .returning::<UserWithScore>()
        .order_by(User::COLUMNS.score.desc())
        .all(&pool)
        .await
        .unwrap();

    assert_eq!(summaries.len(), 3); // Eve(95), Bob(92), Alice(85.5)
    assert_eq!(summaries[0].name, "Eve"); // highest score
}

// ==========================================
// Delete 测试
// ==========================================

#[tokio::test]
async fn test_delete_single() {
    let pool = setup_test_db().await;
    let pks = insert_test_users(&pool).await;

    let rows = User::delete()
        .filter(User::COLUMNS.id.eq(pks[0]))
        .execute(&pool)
        .await
        .unwrap();

    assert_eq!(rows, 1);
    assert!(User::find_by_pk(&pks[0], &pool).await.unwrap().is_none());
}

#[tokio::test]
async fn test_delete_multiple() {
    let pool = setup_test_db().await;
    insert_test_users(&pool).await;

    let rows = User::delete()
        .filter(User::COLUMNS.is_active.eq(false))
        .execute(&pool)
        .await
        .unwrap();

    assert_eq!(rows, 1); // Only Charlie is inactive

    let remaining = User::select().all(&pool).await.unwrap();
    assert_eq!(remaining.len(), 4);
}

#[tokio::test]
async fn test_delete_with_complex_filter() {
    let pool = setup_test_db().await;
    insert_test_users(&pool).await;

    let rows = User::delete()
        .filter(User::COLUMNS.age.lt(23) | User::COLUMNS.age.gt(32))
        .execute(&pool)
        .await
        .unwrap();

    // Diana(22) and Charlie(35)
    assert_eq!(rows, 2);
}

// ==========================================
// Update 测试
// ==========================================

#[tokio::test]
async fn test_update_single_field() {
    let pool = setup_test_db().await;
    let pks = insert_test_users(&pool).await;

    let rows = User::update_query()
        .set(User::COLUMNS.name, "Updated Alice".to_string())
        .filter(User::COLUMNS.id.eq(pks[0]))
        .execute(&pool)
        .await
        .unwrap();

    assert_eq!(rows, 1);

    let user = User::fetch_one_by_pk(&pks[0], &pool).await.unwrap();
    assert_eq!(user.name, "Updated Alice");
}

#[tokio::test]
async fn test_update_multiple_fields() {
    let pool = setup_test_db().await;
    let pks = insert_test_users(&pool).await;

    let rows = User::update_query()
        .set(User::COLUMNS.name, "New Name".to_string())
        .set(User::COLUMNS.email, "new@email.com".to_string())
        .set(User::COLUMNS.age, 99)
        .filter(User::COLUMNS.id.eq(pks[0]))
        .execute(&pool)
        .await
        .unwrap();

    assert_eq!(rows, 1);

    let user = User::fetch_one_by_pk(&pks[0], &pool).await.unwrap();
    assert_eq!(user.name, "New Name");
    assert_eq!(user.email, "new@email.com");
    assert_eq!(user.age, 99);
}

#[tokio::test]
async fn test_update_multiple_rows() {
    let pool = setup_test_db().await;
    insert_test_users(&pool).await;

    let rows = User::update_query()
        .set(User::COLUMNS.is_active, false)
        .filter(User::COLUMNS.age.gt(28))
        .execute(&pool)
        .await
        .unwrap();

    // Bob(30), Charlie(35)
    assert_eq!(rows, 2);

    let inactive = User::select()
        .filter(User::COLUMNS.is_active.eq(false))
        .all(&pool)
        .await
        .unwrap();
    // Charlie was already inactive + Bob now inactive = at least 2
    assert!(inactive.len() >= 2);
}

// ==========================================
// 批量操作测试
// ==========================================

#[tokio::test]
async fn test_insert_many_returning_pks() {
    let pool = setup_test_db().await;

    let pks = User::insert_many(vec![
        CreateUser {
            name: "a".into(),
            email: "a@test.com".into(),
            age: 20,
            score: 60.0,
            is_active: true,
        },
        CreateUser {
            name: "b".into(),
            email: "b@test.com".into(),
            age: 21,
            score: 61.0,
            is_active: true,
        },
        CreateUser {
            name: "c".into(),
            email: "c@test.com".into(),
            age: 22,
            score: 62.0,
            is_active: true,
        },
    ])
    .returning_pk(&pool)
    .await
    .unwrap();

    assert_eq!(pks.len(), 3);
    assert!(pks[0] < pks[1] && pks[1] < pks[2]);
}

#[tokio::test]
async fn test_insert_many_returning_entities() {
    let pool = setup_test_db().await;

    let users = User::insert_many(vec![
        CreateUser {
            name: "x".into(),
            email: "x@test.com".into(),
            age: 30,
            score: 80.0,
            is_active: true,
        },
        CreateUser {
            name: "y".into(),
            email: "y@test.com".into(),
            age: 31,
            score: 81.0,
            is_active: false,
        },
    ])
    .returning_entity(&pool)
    .await
    .unwrap();

    assert_eq!(users.len(), 2);
    assert_eq!(users[0].name, "x");
    assert_eq!(users[1].name, "y");
}

// ==========================================
// 事务测试
// ==========================================

#[tokio::test]
async fn test_transaction_commit() {
    let pool = setup_test_db().await;

    let mut tx = pool.begin().await.unwrap();

    let pk = CreateUser {
        name: "tx_user".into(),
        email: "tx@test.com".into(),
        age: 25,
        score: 70.0,
        is_active: true,
    }
    .insert::<User>()
    .returning_pk(&mut *tx)
    .await
    .unwrap();

    tx.commit().await.unwrap();

    // Should be visible after commit
    let user = User::find_by_pk(&pk, &pool).await.unwrap();
    assert!(user.is_some());
}

#[tokio::test]
async fn test_transaction_rollback() {
    let pool = setup_test_db().await;

    let mut tx = pool.begin().await.unwrap();

    let pk = CreateUser {
        name: "rollback_user".into(),
        email: "rollback@test.com".into(),
        age: 25,
        score: 70.0,
        is_active: true,
    }
    .insert::<User>()
    .returning_pk(&mut *tx)
    .await
    .unwrap();

    tx.rollback().await.unwrap();

    // Should NOT be visible after rollback
    let user = User::find_by_pk(&pk, &pool).await.unwrap();
    assert!(user.is_none());
}

#[tokio::test]
async fn test_transaction_multiple_operations() {
    let pool = setup_test_db().await;
    let pks = insert_test_users(&pool).await;

    let mut tx = pool.begin().await.unwrap();

    // Update in transaction
    User::update_query()
        .set(User::COLUMNS.name, "TxUpdated".to_string())
        .filter(User::COLUMNS.id.eq(pks[0]))
        .execute(&mut *tx)
        .await
        .unwrap();

    // Delete in same transaction
    User::delete()
        .filter(User::COLUMNS.id.eq(pks[1]))
        .execute(&mut *tx)
        .await
        .unwrap();

    // Insert in same transaction
    CreateUser {
        name: "TxNew".into(),
        email: "txnew@test.com".into(),
        age: 50,
        score: 50.0,
        is_active: true,
    }
    .insert::<User>()
    .returning_pk(&mut *tx)
    .await
    .unwrap();

    tx.commit().await.unwrap();

    // Verify all changes
    let updated = User::fetch_one_by_pk(&pks[0], &pool).await.unwrap();
    assert_eq!(updated.name, "TxUpdated");

    let deleted = User::find_by_pk(&pks[1], &pool).await.unwrap();
    assert!(deleted.is_none());

    let all = User::select().all(&pool).await.unwrap();
    assert_eq!(all.len(), 5); // 5 original - 1 deleted + 1 new = 5
}

// ==========================================
// 浮点数精度测试
// ==========================================

#[tokio::test]
async fn test_float_comparison() {
    let pool = setup_test_db().await;
    insert_test_users(&pool).await;

    // Test float equality (be careful with floating point)
    let users = User::select()
        .filter(User::COLUMNS.score.gte(85.0))
        .filter(User::COLUMNS.score.lte(95.0))
        .order_by(User::COLUMNS.score)
        .all(&pool)
        .await
        .unwrap();

    // Alice(85.5), Bob(92.0), Eve(95.0)
    assert_eq!(users.len(), 3);
}

// ==========================================
// 复合主键场景（通过复杂 filter 模拟）
// ==========================================

#[tokio::test]
async fn test_composite_condition_as_unique_identifier() {
    let pool = setup_test_db().await;
    insert_test_users(&pool).await;

    // Find user by name AND email (like a composite unique constraint)
    let user = User::select()
        .filter(
            User::COLUMNS.name.eq("Alice".to_string())
                & User::COLUMNS.email.eq("alice@test.com".to_string()),
        )
        .optional(&pool)
        .await
        .unwrap();

    assert!(user.is_some());
    assert_eq!(user.unwrap().age, 25);
}

// ==========================================
// 大数据量测试
// ==========================================

#[tokio::test]
async fn test_bulk_insert_and_query() {
    let pool = setup_test_db().await;

    // Insert 100 users
    let mut users = Vec::new();
    for i in 0..100 {
        users.push(CreateUser {
            name: format!("user_{}", i),
            email: format!("user_{}@test.com", i),
            age: 20 + (i % 50) as i32,
            score: 50.0 + (i as f64),
            is_active: i % 2 == 0,
        });
    }

    let pks = User::insert_many(users).returning_pk(&pool).await.unwrap();
    assert_eq!(pks.len(), 100);

    // Query with complex conditions
    let result = User::select()
        .filter(User::COLUMNS.is_active.eq(true))
        .filter(User::COLUMNS.age.between(30, 40))
        .order_by(User::COLUMNS.score.desc())
        .limit(10)
        .all(&pool)
        .await
        .unwrap();

    assert!(result.len() <= 10);
    assert!(result.iter().all(|u| u.is_active && u.age >= 30 && u.age <= 40));
}

// ==========================================
// 日期时间类型操作符测试
// ==========================================

/// 插入带有不同时间戳的产品用于测试
async fn insert_test_products_with_dates(pool: &PgPool) -> Vec<i32> {
    use chrono::{Duration, Utc};

    let base_time = Utc::now();
    let mut pks = Vec::new();

    let products = vec![
        CreateProduct {
            uuid: uuid::Uuid::new_v4(),
            name: "Product A".into(),
            price: "10.00".parse().unwrap(),
            metadata: serde_json::json!({}),
            created_at: base_time - Duration::days(30), // 30天前
        },
        CreateProduct {
            uuid: uuid::Uuid::new_v4(),
            name: "Product B".into(),
            price: "20.00".parse().unwrap(),
            metadata: serde_json::json!({}),
            created_at: base_time - Duration::days(7), // 7天前
        },
        CreateProduct {
            uuid: uuid::Uuid::new_v4(),
            name: "Product C".into(),
            price: "30.00".parse().unwrap(),
            metadata: serde_json::json!({}),
            created_at: base_time - Duration::days(1), // 昨天
        },
        CreateProduct {
            uuid: uuid::Uuid::new_v4(),
            name: "Product D".into(),
            price: "40.00".parse().unwrap(),
            metadata: serde_json::json!({}),
            created_at: base_time, // 现在
        },
    ];

    for product in products {
        let pk = product
            .insert::<Product>()
            .returning_pk(pool)
            .await
            .unwrap();
        pks.push(pk);
    }
    pks
}

#[tokio::test]
async fn test_datetime_gt_operator() {
    use chrono::{Duration, Utc};
    let pool = setup_test_db().await;
    insert_test_products_with_dates(&pool).await;

    // 查找 7 天内创建的产品
    let cutoff = Utc::now() - Duration::days(7);
    let products = Product::select()
        .filter(Product::COLUMNS.created_at.gt(cutoff))
        .all(&pool)
        .await
        .unwrap();

    // Product C (昨天) 和 Product D (现在)
    assert_eq!(products.len(), 2);
}

#[tokio::test]
async fn test_datetime_lt_operator() {
    use chrono::{Duration, Utc};
    let pool = setup_test_db().await;
    insert_test_products_with_dates(&pool).await;

    // 查找 14 天前创建的产品 (确保只有 Product A)
    let cutoff = Utc::now() - Duration::days(14);
    let products = Product::select()
        .filter(Product::COLUMNS.created_at.lt(cutoff))
        .all(&pool)
        .await
        .unwrap();

    // Product A (30天前)
    assert_eq!(products.len(), 1);
    assert_eq!(products[0].name, "Product A");
}

#[tokio::test]
async fn test_datetime_gte_operator() {
    use chrono::{Duration, Utc};
    let pool = setup_test_db().await;
    insert_test_products_with_dates(&pool).await;

    // 查找 1 天内创建的产品 (包含边界)
    let cutoff = Utc::now() - Duration::days(2);
    let products = Product::select()
        .filter(Product::COLUMNS.created_at.gte(cutoff))
        .all(&pool)
        .await
        .unwrap();

    // Product C (昨天) 和 Product D (现在)
    assert_eq!(products.len(), 2);
}

#[tokio::test]
async fn test_datetime_lte_operator() {
    use chrono::{Duration, Utc};
    let pool = setup_test_db().await;
    insert_test_products_with_dates(&pool).await;

    // 查找 7 天前及更早创建的产品
    let cutoff = Utc::now() - Duration::days(7);
    let products = Product::select()
        .filter(Product::COLUMNS.created_at.lte(cutoff))
        .all(&pool)
        .await
        .unwrap();

    // Product A (30天前) 和 Product B (刚好7天前)
    assert_eq!(products.len(), 2);
}

#[tokio::test]
async fn test_datetime_between_operator() {
    use chrono::{Duration, Utc};
    let pool = setup_test_db().await;
    insert_test_products_with_dates(&pool).await;

    // 查找 7-14 天前创建的产品
    let start = Utc::now() - Duration::days(14);
    let end = Utc::now() - Duration::days(2);
    let products = Product::select()
        .filter(Product::COLUMNS.created_at.between(start, end))
        .all(&pool)
        .await
        .unwrap();

    // Product B (7天前)
    assert_eq!(products.len(), 1);
    assert_eq!(products[0].name, "Product B");
}

#[tokio::test]
async fn test_datetime_order_by() {
    let pool = setup_test_db().await;
    insert_test_products_with_dates(&pool).await;

    // 按创建时间升序排序
    let products = Product::select()
        .order_by(Product::COLUMNS.created_at)
        .all(&pool)
        .await
        .unwrap();

    assert_eq!(products.len(), 4);
    assert_eq!(products[0].name, "Product A"); // 最早
    assert_eq!(products[3].name, "Product D"); // 最新

    // 按创建时间降序排序
    let products_desc = Product::select()
        .order_by(Product::COLUMNS.created_at.desc())
        .all(&pool)
        .await
        .unwrap();

    assert_eq!(products_desc[0].name, "Product D"); // 最新
    assert_eq!(products_desc[3].name, "Product A"); // 最早
}

#[tokio::test]
async fn test_datetime_combined_with_other_filters() {
    use chrono::{Duration, Utc};
    let pool = setup_test_db().await;
    insert_test_products_with_dates(&pool).await;

    // 查找价格 > 15 且 7 天内创建的产品
    let cutoff = Utc::now() - Duration::days(7);
    let price: sqlx::types::BigDecimal = "15.00".parse().unwrap();

    let products = Product::select()
        .filter(Product::COLUMNS.price.gt(price) & Product::COLUMNS.created_at.gt(cutoff))
        .all(&pool)
        .await
        .unwrap();

    // Product C (30.00, 昨天) 和 Product D (40.00, 现在)
    assert_eq!(products.len(), 2);
}

#[tokio::test]
async fn test_datetime_in_update() {
    use chrono::{Duration, Utc};
    let pool = setup_test_db().await;
    let pks = insert_test_products_with_dates(&pool).await;

    // 更新 14 天前的产品名称 (确保只有 Product A)
    let cutoff = Utc::now() - Duration::days(14);
    let rows = Product::update_query()
        .set(Product::COLUMNS.name, "Old Product".to_string())
        .filter(Product::COLUMNS.created_at.lt(cutoff))
        .execute(&pool)
        .await
        .unwrap();

    assert_eq!(rows, 1); // 只有 Product A

    let product = Product::fetch_one_by_pk(&pks[0], &pool).await.unwrap();
    assert_eq!(product.name, "Old Product");
}

#[tokio::test]
async fn test_datetime_in_delete() {
    use chrono::{Duration, Utc};
    let pool = setup_test_db().await;
    insert_test_products_with_dates(&pool).await;

    // 删除 14 天前的产品
    let cutoff = Utc::now() - Duration::days(14);
    let rows = Product::delete()
        .filter(Product::COLUMNS.created_at.lt(cutoff))
        .execute(&pool)
        .await
        .unwrap();

    assert_eq!(rows, 1); // 只有 Product A (30天前)

    let remaining = Product::select().all(&pool).await.unwrap();
    assert_eq!(remaining.len(), 3);
}
