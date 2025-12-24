//! Integration tests for Executor trait implementations on reference types
//!
//! This test suite ensures that &PooledConnection, &Connection, and &Transaction
//! all properly implement the Executor trait and can be used interchangeably
//! with their owned counterparts.

use conservator::{Creatable, Domain, Executor, PooledConnection};
use deadpool_postgres::{Config, PoolConfig};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU32, Ordering};
use testcontainers::{Container, clients::Cli};
use testcontainers_modules::postgres::Postgres;

// Shared Docker client and container
static DOCKER: OnceLock<Cli> = OnceLock::new();
static POSTGRES_CONTAINER: OnceLock<Container<'static, Postgres>> = OnceLock::new();
static DB_COUNTER: AtomicU32 = AtomicU32::new(0);

fn docker() -> &'static Cli {
    DOCKER.get_or_init(Cli::default)
}

fn postgres_container() -> &'static Container<'static, Postgres> {
    POSTGRES_CONTAINER.get_or_init(|| {
        let docker = docker();
        docker.run(Postgres::default())
    })
}

fn unique_db_name() -> String {
    let count = DB_COUNTER.fetch_add(1, Ordering::SeqCst);
    format!("executor_refs_test_db_{}", count)
}

async fn create_test_pool() -> PooledConnection {
    let container = postgres_container();
    let port = container.get_host_port_ipv4(5432);

    let db_name = unique_db_name();

    // Create database using admin connection
    let mut admin_config = Config::new();
    admin_config.host = Some("localhost".to_string());
    admin_config.port = Some(port);
    admin_config.user = Some("postgres".to_string());
    admin_config.password = Some("postgres".to_string());
    admin_config.dbname = Some("postgres".to_string());
    admin_config.pool = Some(PoolConfig {
        max_size: 2,
        ..Default::default()
    });

    let admin_pool = PooledConnection::from_config(admin_config).unwrap();
    admin_pool
        .get()
        .await
        .unwrap()
        .execute(&format!("CREATE DATABASE {}", db_name), &[])
        .await
        .unwrap();

    // Create pool for the new database
    let mut config = Config::new();
    config.host = Some("localhost".to_string());
    config.port = Some(port);
    config.user = Some("postgres".to_string());
    config.password = Some("postgres".to_string());
    config.dbname = Some(db_name);
    config.pool = Some(PoolConfig {
        max_size: 2,
        ..Default::default()
    });

    let pool = PooledConnection::from_config(config).unwrap();

    let conn = pool.get().await.unwrap();
    conn.client()
        .batch_execute(
            "CREATE TABLE users (
                id SERIAL PRIMARY KEY,
                name TEXT NOT NULL,
                email TEXT NOT NULL
            )",
        )
        .await
        .unwrap();

    pool
}

#[derive(Debug, Domain)]
#[domain(table = "users")]
struct User {
    #[domain(primary_key)]
    id: i32,
    name: String,
    email: String,
}

#[derive(Debug, Creatable)]
struct CreateUser {
    name: String,
    email: String,
}

/// Test that &PooledConnection implements Executor
#[tokio::test]
async fn test_pooled_connection_reference_as_executor() {
    let pool = create_test_pool().await;

    // Use &PooledConnection directly
    let user = CreateUser {
        name: "Alice".to_string(),
        email: "alice@example.com".to_string(),
    }
    .insert::<User>()
    .returning_entity(&pool) // Passing &PooledConnection
    .await
    .unwrap();

    assert_eq!(user.name, "Alice");
    assert_eq!(user.email, "alice@example.com");

    // Query using &PooledConnection
    let found = User::select()
        .filter(User::COLUMNS.id.eq(user.id))
        .one(&pool) // Passing &PooledConnection
        .await
        .unwrap();

    assert_eq!(found.id, user.id);
    assert_eq!(found.name, "Alice");
}

/// Test that &Connection implements Executor
#[tokio::test]
async fn test_connection_reference_as_executor() {
    let pool = create_test_pool().await;
    let conn = pool.get().await.unwrap();

    // Use &Connection directly
    let user = CreateUser {
        name: "Bob".to_string(),
        email: "bob@example.com".to_string(),
    }
    .insert::<User>()
    .returning_entity(&conn) // Passing &Connection
    .await
    .unwrap();

    assert_eq!(user.name, "Bob");

    // Query using &Connection
    let found = User::select()
        .filter(User::COLUMNS.id.eq(user.id))
        .one(&conn) // Passing &Connection
        .await
        .unwrap();

    assert_eq!(found.id, user.id);
}

/// Test that &Transaction implements Executor
#[tokio::test]
async fn test_transaction_reference_as_executor() {
    let pool = create_test_pool().await;
    let mut conn = pool.get().await.unwrap();
    let tx = conn.begin().await.unwrap();

    // Use &Transaction directly
    let user = CreateUser {
        name: "Charlie".to_string(),
        email: "charlie@example.com".to_string(),
    }
    .insert::<User>()
    .returning_entity(&tx) // Passing &Transaction
    .await
    .unwrap();

    assert_eq!(user.name, "Charlie");

    // Query using &Transaction
    let found = User::select()
        .filter(User::COLUMNS.id.eq(user.id))
        .one(&tx) // Passing &Transaction
        .await
        .unwrap();

    assert_eq!(found.id, user.id);

    tx.commit().await.unwrap();

    // Verify after commit
    let verified = User::fetch_one_by_pk(&user.id, &pool).await.unwrap();
    assert_eq!(verified.name, "Charlie");
}

/// Test that execute() works with reference types
#[tokio::test]
async fn test_execute_with_references() {
    let pool = create_test_pool().await;

    // Insert using &PooledConnection
    let user = CreateUser {
        name: "Dave".to_string(),
        email: "dave@example.com".to_string(),
    }
    .insert::<User>()
    .returning_entity(&pool)
    .await
    .unwrap();

    // Update using &PooledConnection
    let rows = User::update()
        .set(User::COLUMNS.name, "David".to_string())
        .filter(User::COLUMNS.id.eq(user.id))
        .execute(&pool) // Passing &PooledConnection
        .await
        .unwrap();

    assert_eq!(rows, 1);

    // Verify with &Connection
    let conn = pool.get().await.unwrap();
    let updated = User::fetch_one_by_pk(&user.id, &conn).await.unwrap();
    assert_eq!(updated.name, "David");
}

/// Test that query_scalar() works with reference types
#[tokio::test]
async fn test_query_scalar_with_references() {
    let pool = create_test_pool().await;

    // Insert test data
    for i in 1..=5 {
        CreateUser {
            name: format!("User{}", i),
            email: format!("user{}@example.com", i),
        }
        .insert::<User>()
        .returning_pk(&pool)
        .await
        .unwrap();
    }

    // Count using &PooledConnection
    let count: i64 = pool
        .query_scalar("SELECT COUNT(*) FROM users", &[])
        .await
        .unwrap();

    assert_eq!(count, 5);
}

/// Test that query_opt() works with reference types
#[tokio::test]
async fn test_query_opt_with_references() {
    let pool = create_test_pool().await;
    let conn = pool.get().await.unwrap();

    // Insert test data
    let user = CreateUser {
        name: "Eve".to_string(),
        email: "eve@example.com".to_string(),
    }
    .insert::<User>()
    .returning_entity(&conn)
    .await
    .unwrap();

    // Query existing user using &Connection
    let found = User::select()
        .filter(User::COLUMNS.id.eq(user.id))
        .optional(&conn) // Passing &Connection
        .await
        .unwrap();

    assert!(found.is_some());

    // Query non-existing user using &Connection
    let not_found = User::select()
        .filter(User::COLUMNS.id.eq(99999))
        .optional(&conn) // Passing &Connection
        .await
        .unwrap();

    assert!(not_found.is_none());
}

/// Test complex operations with mixed reference and owned types
#[tokio::test]
async fn test_mixed_executor_types() {
    let pool = create_test_pool().await;

    // Insert with owned PooledConnection (implicit &PooledConnection)
    let user1 = CreateUser {
        name: "Frank".to_string(),
        email: "frank@example.com".to_string(),
    }
    .insert::<User>()
    .returning_entity(&pool)
    .await
    .unwrap();

    // Get owned Connection
    let conn = pool.get().await.unwrap();

    // Query with owned Connection (implicit &Connection)
    let user2 = User::select()
        .filter(User::COLUMNS.id.eq(user1.id))
        .one(&conn)
        .await
        .unwrap();

    assert_eq!(user1.id, user2.id);

    // Start transaction with borrowed Connection
    let mut conn_borrowed = pool.get().await.unwrap();
    let tx = conn_borrowed.begin().await.unwrap();

    // Update with borrowed Transaction
    User::update()
        .set(User::COLUMNS.name, "Franklin".to_string())
        .filter(User::COLUMNS.id.eq(user1.id))
        .execute(&tx)
        .await
        .unwrap();

    tx.commit().await.unwrap();

    // Verify with reference
    let updated = User::fetch_one_by_pk(&user1.id, &pool).await.unwrap();
    assert_eq!(updated.name, "Franklin");
}
