//! SQL macro integration tests
//!
//! Tests all 5 action types (fetch, exists, find, fetch_all, execute) with real database.

use conservator::{sql, Domain, Executor, PooledConnection, Selectable};
use deadpool_postgres::{Config, PoolConfig};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::OnceLock;
use testcontainers::{clients::Cli, Container};
use testcontainers_modules::postgres::Postgres;

// Shared Docker client and container
static DOCKER: OnceLock<Cli> = OnceLock::new();
static CONTAINER: OnceLock<Container<'static, Postgres>> = OnceLock::new();
static DB_COUNTER: AtomicU32 = AtomicU32::new(0);

fn get_container() -> &'static Container<'static, Postgres> {
    let docker = DOCKER.get_or_init(Cli::default);
    CONTAINER.get_or_init(|| docker.run(Postgres::default()))
}

async fn setup_test_db() -> PooledConnection {
    let container = get_container();
    let port = container.get_host_port_ipv4(5432);
    let db_num = DB_COUNTER.fetch_add(1, Ordering::SeqCst);
    let db_name = format!("sql_macro_test_{}", db_num);

    // Create database
    // Use small pool size (2) for tests to avoid "too many clients" error in CI
    let mut admin_config = Config::new();
    admin_config.host = Some("127.0.0.1".to_string());
    admin_config.port = Some(port);
    admin_config.user = Some("postgres".to_string());
    admin_config.password = Some("postgres".to_string());
    admin_config.dbname = Some("postgres".to_string());
    admin_config.pool = Some(PoolConfig {
        max_size: 2,
        ..Default::default()
    });
    let admin_pool = conservator::PooledConnection::from_config(admin_config).unwrap();

    let client = admin_pool.get().await.unwrap();
    client
        .execute(&format!("CREATE DATABASE {}", db_name), &[])
        .await
        .unwrap();

    // Connect to new database
    // Use small pool size (2) for tests to avoid "too many clients" error in CI
    let mut test_config = Config::new();
    test_config.host = Some("127.0.0.1".to_string());
    test_config.port = Some(port);
    test_config.user = Some("postgres".to_string());
    test_config.password = Some("postgres".to_string());
    test_config.dbname = Some(db_name.clone());
    test_config.pool = Some(PoolConfig {
        max_size: 2,
        ..Default::default()
    });
    let pool = conservator::PooledConnection::from_config(test_config).unwrap();

    // Create test table
    let client = pool.get().await.unwrap();
    client
        .execute(
            "CREATE TABLE users (
                id SERIAL PRIMARY KEY,
                name TEXT NOT NULL,
                email TEXT NOT NULL UNIQUE,
                active BOOLEAN NOT NULL DEFAULT true
            )",
            &[],
        )
        .await
        .unwrap();

    pool
}

#[derive(Debug, Clone, Domain)]
#[domain(table = "users")]
struct User {
    #[domain(primary_key)]
    id: i32,
    name: String,
    email: String,
    active: bool,
}

// Test fetch action
#[sql(fetch)]
pub async fn fetch_user_by_id(id: i32) -> User {
    "SELECT id, name, email, active FROM users WHERE id = :id"
}

// Test fetch action with multiple parameters
#[sql(fetch)]
pub async fn fetch_user_by_name_and_email(name: &str, email: &str) -> User {
    "SELECT id, name, email, active FROM users WHERE name = :name AND email = :email"
}

// Test exists action
#[sql(exists)]
pub async fn user_exists(email: &str) -> bool {
    "SELECT 1 FROM users WHERE email = :email"
}

// Test exists action with no parameters
#[sql(exists)]
pub async fn has_users() -> bool {
    "SELECT 1 FROM users"
}

// Test find action
#[sql(find)]
pub async fn find_user_by_email(email: &str) -> Option<User> {
    "SELECT id, name, email, active FROM users WHERE email = :email"
}

// Test find action with no results
#[sql(find)]
pub async fn find_inactive_user(id: i32) -> Option<User> {
    "SELECT id, name, email, active FROM users WHERE id = :id AND active = false"
}

// Test fetch_all action
#[sql(fetch_all)]
pub async fn list_all_users() -> Vec<User> {
    "SELECT id, name, email, active FROM users ORDER BY id"
}

// Test fetch_all with filter
#[sql(fetch_all)]
pub async fn list_users_by_name(name: &str) -> Vec<User> {
    "SELECT id, name, email, active FROM users WHERE name = :name ORDER BY id"
}

// Test execute action
#[sql(execute)]
pub async fn update_user_name(id: i32, name: &str) -> () {
    "UPDATE users SET name = :name WHERE id = :id"
}

// Test execute action with delete
#[sql(execute)]
pub async fn delete_user(id: i32) -> () {
    "DELETE FROM users WHERE id = :id"
}

#[tokio::test]
async fn test_fetch_action_single_param() {
    let pool = setup_test_db().await;

    // Insert test data
    let client = pool.get().await.unwrap();
    client
        .execute(
            "INSERT INTO users (name, email, active) VALUES ('Alice', 'alice@example.com', true)",
            &[],
        )
        .await
        .unwrap();

    // Test fetch
    let user = fetch_user_by_id(1, &client).await.unwrap();
    assert_eq!(user.id, 1);
    assert_eq!(user.name, "Alice");
    assert_eq!(user.email, "alice@example.com");
    assert!(user.active);
}

#[tokio::test]
async fn test_fetch_action_multiple_params() {
    let pool = setup_test_db().await;

    // Insert test data
    let client = pool.get().await.unwrap();
    client
        .execute(
            "INSERT INTO users (name, email, active) VALUES ('Bob', 'bob@example.com', true)",
            &[],
        )
        .await
        .unwrap();

    // Test fetch with multiple parameters
    let user = fetch_user_by_name_and_email("Bob", "bob@example.com", &client)
        .await
        .unwrap();
    assert_eq!(user.name, "Bob");
    assert_eq!(user.email, "bob@example.com");
}

#[tokio::test]
async fn test_fetch_action_not_found() {
    let pool = setup_test_db().await;
    let client = pool.get().await.unwrap();

    // Should return error when not found
    let result = fetch_user_by_id(999, &client).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_exists_action_true() {
    let pool = setup_test_db().await;

    // Insert test data
    let client = pool.get().await.unwrap();
    client
        .execute(
            "INSERT INTO users (name, email, active) VALUES ('Charlie', 'charlie@example.com', true)",
            &[],
        )
        .await
        .unwrap();

    // Test exists - should be true
    let exists = user_exists("charlie@example.com", &client).await.unwrap();
    assert!(exists);
}

#[tokio::test]
async fn test_exists_action_false() {
    let pool = setup_test_db().await;
    let client = pool.get().await.unwrap();

    // Test exists - should be false
    let exists = user_exists("nonexistent@example.com", &client)
        .await
        .unwrap();
    assert!(!exists);
}

#[tokio::test]
async fn test_exists_action_no_params() {
    let pool = setup_test_db().await;
    let client = pool.get().await.unwrap();

    // No users yet
    let exists = has_users(&client).await.unwrap();
    assert!(!exists);

    // Insert a user
    client
        .execute(
            "INSERT INTO users (name, email, active) VALUES ('Dave', 'dave@example.com', true)",
            &[],
        )
        .await
        .unwrap();

    // Now should be true
    let exists = has_users(&client).await.unwrap();
    assert!(exists);
}

#[tokio::test]
async fn test_find_action_some() {
    let pool = setup_test_db().await;

    // Insert test data
    let client = pool.get().await.unwrap();
    client
        .execute(
            "INSERT INTO users (name, email, active) VALUES ('Eve', 'eve@example.com', true)",
            &[],
        )
        .await
        .unwrap();

    // Test find - should return Some
    let user = find_user_by_email("eve@example.com", &client)
        .await
        .unwrap();
    assert!(user.is_some());
    let user = user.unwrap();
    assert_eq!(user.name, "Eve");
}

#[tokio::test]
async fn test_find_action_none() {
    let pool = setup_test_db().await;
    let client = pool.get().await.unwrap();

    // Test find - should return None
    let user = find_user_by_email("nobody@example.com", &client)
        .await
        .unwrap();
    assert!(user.is_none());
}

#[tokio::test]
async fn test_find_action_with_filter_no_match() {
    let pool = setup_test_db().await;

    // Insert active user
    let client = pool.get().await.unwrap();
    client
        .execute(
            "INSERT INTO users (name, email, active) VALUES ('Frank', 'frank@example.com', true)",
            &[],
        )
        .await
        .unwrap();

    // Test find inactive user - should return None (user is active)
    let user = find_inactive_user(1, &client).await.unwrap();
    assert!(user.is_none());
}

#[tokio::test]
async fn test_fetch_all_action_multiple_rows() {
    let pool = setup_test_db().await;

    // Insert test data
    let client = pool.get().await.unwrap();
    client
        .execute(
            "INSERT INTO users (name, email, active) VALUES
                ('Grace', 'grace@example.com', true),
                ('Henry', 'henry@example.com', true),
                ('Iris', 'iris@example.com', false)",
            &[],
        )
        .await
        .unwrap();

    // Test fetch_all
    let users = list_all_users(&client).await.unwrap();
    assert_eq!(users.len(), 3);
    assert_eq!(users[0].name, "Grace");
    assert_eq!(users[1].name, "Henry");
    assert_eq!(users[2].name, "Iris");
}

#[tokio::test]
async fn test_fetch_all_action_empty() {
    let pool = setup_test_db().await;
    let client = pool.get().await.unwrap();

    // Test fetch_all on empty table
    let users = list_all_users(&client).await.unwrap();
    assert_eq!(users.len(), 0);
}

#[tokio::test]
async fn test_fetch_all_action_with_filter() {
    let pool = setup_test_db().await;

    // Insert test data
    let client = pool.get().await.unwrap();
    client
        .execute(
            "INSERT INTO users (name, email, active) VALUES
                ('John', 'john1@example.com', true),
                ('John', 'john2@example.com', true),
                ('Jane', 'jane@example.com', true)",
            &[],
        )
        .await
        .unwrap();

    // Test fetch_all with filter
    let users = list_users_by_name("John", &client).await.unwrap();
    assert_eq!(users.len(), 2);
    assert_eq!(users[0].name, "John");
    assert_eq!(users[1].name, "John");
}

#[tokio::test]
async fn test_execute_action_update() {
    let pool = setup_test_db().await;

    // Insert test data
    let client = pool.get().await.unwrap();
    client
        .execute(
            "INSERT INTO users (name, email, active) VALUES ('Kate', 'kate@example.com', true)",
            &[],
        )
        .await
        .unwrap();

    // Test execute (update)
    update_user_name(1, "Katherine", &client).await.unwrap();

    // Verify update
    let user = fetch_user_by_id(1, &client).await.unwrap();
    assert_eq!(user.name, "Katherine");
}

#[tokio::test]
async fn test_execute_action_delete() {
    let pool = setup_test_db().await;

    // Insert test data
    let client = pool.get().await.unwrap();
    client
        .execute(
            "INSERT INTO users (name, email, active) VALUES ('Leo', 'leo@example.com', true)",
            &[],
        )
        .await
        .unwrap();

    // Verify exists
    let exists = user_exists("leo@example.com", &client).await.unwrap();
    assert!(exists);

    // Test execute (delete)
    delete_user(1, &client).await.unwrap();

    // Verify deleted
    let exists = user_exists("leo@example.com", &client).await.unwrap();
    assert!(!exists);
}
