//! Migration system integration tests

use conservator::{AppliedInfo, MigrateReport, Migration, Migrator, PooledConnection};
use deadpool_postgres::{Config, PoolConfig};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU32, Ordering};
use testcontainers::{Container, clients::Cli};
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
    let db_name = format!("migrate_integration_test_{}", db_num);

    // Create database
    // Use small pool size (2) for tests to avoid "too many clients" error in CI
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
    let conn = admin_pool.get().await.unwrap();

    use std::ops::Deref;
    let client: &tokio_postgres::Client = conn.client().deref();
    let _ = client
        .execute(&format!("DROP DATABASE IF EXISTS \"{}\"", db_name), &[])
        .await;
    client
        .execute(&format!("CREATE DATABASE \"{}\"", db_name), &[])
        .await
        .unwrap();
    drop(conn);

    // Connect to test database
    // Use small pool size (2) for tests to avoid "too many clients" error in CI
    let mut test_config = Config::new();
    test_config.host = Some("localhost".to_string());
    test_config.port = Some(port);
    test_config.user = Some("postgres".to_string());
    test_config.password = Some("postgres".to_string());
    test_config.dbname = Some(db_name);
    test_config.pool = Some(PoolConfig {
        max_size: 2,
        ..Default::default()
    });
    PooledConnection::from_config(test_config).unwrap()
}

#[tokio::test]
async fn test_empty_migrator() {
    let pool = setup_test_db().await;
    let mut conn = pool.get().await.unwrap();

    let migrator = Migrator::new();
    let report = migrator.run(&mut conn).await.unwrap();

    assert_eq!(report.skipped, 0);
    assert_eq!(report.applied.len(), 0);
}

#[tokio::test]
async fn test_single_migration() {
    let pool = setup_test_db().await;
    let mut conn = pool.get().await.unwrap();

    let mut migrator = Migrator::new();
    migrator.add_migration(Migration::new(
        1,
        "create users table",
        "CREATE TABLE users (id SERIAL PRIMARY KEY, name TEXT NOT NULL)",
    ));

    let report = migrator.run(&mut conn).await.unwrap();

    assert_eq!(report.applied.len(), 1);
    assert_eq!(report.applied[0].version, 1);
    assert_eq!(report.applied[0].description, "create users table");

    // Verify table exists
    use std::ops::Deref;
    let client: &tokio_postgres::Client = conn.client().deref();
    let row = client
        .query_one(
            "SELECT COUNT(*) FROM information_schema.tables WHERE table_name = 'users'",
            &[],
        )
        .await
        .unwrap();
    let count: i64 = row.get(0);
    assert_eq!(count, 1);
}

#[tokio::test]
async fn test_multiple_migrations() {
    let pool = setup_test_db().await;
    let mut conn = pool.get().await.unwrap();

    let mut migrator = Migrator::new();
    migrator.add_migration(Migration::new(
        1,
        "create users",
        "CREATE TABLE users (id SERIAL PRIMARY KEY, name TEXT)",
    ));
    migrator.add_migration(Migration::new(
        2,
        "add email column",
        "ALTER TABLE users ADD COLUMN email TEXT",
    ));
    migrator.add_migration(Migration::new(
        3,
        "create posts",
        "CREATE TABLE posts (id SERIAL PRIMARY KEY, user_id INT REFERENCES users(id))",
    ));

    let report = migrator.run(&mut conn).await.unwrap();

    assert_eq!(report.applied.len(), 3);
    assert_eq!(report.applied[0].version, 1);
    assert_eq!(report.applied[1].version, 2);
    assert_eq!(report.applied[2].version, 3);
}

#[tokio::test]
async fn test_idempotent_migration() {
    let pool = setup_test_db().await;
    let mut conn = pool.get().await.unwrap();

    let mut migrator = Migrator::new();
    migrator.add_migration(Migration::new(
        1,
        "create table",
        "CREATE TABLE test_table (id INT)",
    ));

    // First run
    let report1 = migrator.run(&mut conn).await.unwrap();
    assert_eq!(report1.applied.len(), 1);

    // Second run - should skip
    let report2 = migrator.run(&mut conn).await.unwrap();
    assert_eq!(report2.applied.len(), 0);
    assert_eq!(report2.skipped, 1);
}

#[tokio::test]
async fn test_checksum_mismatch() {
    let pool = setup_test_db().await;
    let mut conn = pool.get().await.unwrap();

    // First run with original migration
    let mut migrator1 = Migrator::new();
    migrator1.add_migration(Migration::new(
        1,
        "create table",
        "CREATE TABLE test_table (id INT)",
    ));
    migrator1.run(&mut conn).await.unwrap();

    // Second run with modified migration
    let mut migrator2 = Migrator::new();
    migrator2.add_migration(Migration::new(
        1,
        "create table",
        "CREATE TABLE test_table (id BIGINT)", // Changed!
    ));

    let result = migrator2.run(&mut conn).await;
    assert!(matches!(
        result,
        Err(conservator::MigrateError::ChecksumMismatch(1))
    ));
}

#[tokio::test]
async fn test_missing_source_migration() {
    let pool = setup_test_db().await;
    let mut conn = pool.get().await.unwrap();

    // Run migrations 1 and 2
    let mut migrator1 = Migrator::new();
    migrator1.add_migration(Migration::new(1, "first", "SELECT 1"));
    migrator1.add_migration(Migration::new(2, "second", "SELECT 2"));
    migrator1.run(&mut conn).await.unwrap();

    // Run with only migration 1 (migration 2 is missing)
    let mut migrator2 = Migrator::new();
    migrator2.add_migration(Migration::new(1, "first", "SELECT 1"));

    let result = migrator2.run(&mut conn).await;
    assert!(matches!(
        result,
        Err(conservator::MigrateError::MissingSource(2))
    ));
}

#[tokio::test]
async fn test_ignore_missing_source() {
    let pool = setup_test_db().await;
    let mut conn = pool.get().await.unwrap();

    // Run migrations 1 and 2
    let mut migrator1 = Migrator::new();
    migrator1.add_migration(Migration::new(1, "first", "SELECT 1"));
    migrator1.add_migration(Migration::new(2, "second", "SELECT 2"));
    migrator1.run(&mut conn).await.unwrap();

    // Run with only migration 1, but ignore missing
    let mut migrator2 = Migrator::new();
    migrator2.set_ignore_missing(true);
    migrator2.add_migration(Migration::new(1, "first", "SELECT 1"));

    let result = migrator2.run(&mut conn).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_migrations_table_created() {
    let pool = setup_test_db().await;
    let mut conn = pool.get().await.unwrap();

    let migrator = Migrator::new();
    migrator.run(&mut conn).await.unwrap();

    // Verify migrations table exists
    use std::ops::Deref;
    let client: &tokio_postgres::Client = conn.client().deref();
    let row = client
        .query_one(
            "SELECT COUNT(*) FROM information_schema.tables WHERE table_name = '_conservator_migrations'",
            &[],
        )
        .await
        .unwrap();
    let count: i64 = row.get(0);
    assert_eq!(count, 1);
}

#[tokio::test]
async fn test_migration_report_display() {
    let report = MigrateReport {
        skipped: 2,
        applied: vec![
            AppliedInfo {
                version: 3,
                description: "add column".to_string(),
                duration: std::time::Duration::from_millis(15),
            },
            AppliedInfo {
                version: 4,
                description: "create index".to_string(),
                duration: std::time::Duration::from_millis(42),
            },
        ],
    };

    let display = format!("{}", report);
    assert!(display.contains("Applied 2 migration(s)"));
    assert!(display.contains("3 - add column"));
    assert!(display.contains("4 - create index"));
}

#[tokio::test]
async fn test_migration_ordering() {
    let pool = setup_test_db().await;
    let mut conn = pool.get().await.unwrap();

    // Add migrations out of order
    let mut migrator = Migrator::new();
    migrator.add_migration(Migration::new(3, "third", "SELECT 3"));
    migrator.add_migration(Migration::new(1, "first", "SELECT 1"));
    migrator.add_migration(Migration::new(2, "second", "SELECT 2"));

    let report = migrator.run(&mut conn).await.unwrap();

    // Should be applied in order
    assert_eq!(report.applied[0].version, 1);
    assert_eq!(report.applied[1].version, 2);
    assert_eq!(report.applied[2].version, 3);
}
