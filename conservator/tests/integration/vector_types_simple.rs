//! Simple integration tests for vector type support
//!
//! Tests Vec<i16>, Vec<i32>, Vec<i64>, Vec<f32>, Vec<f64>, Vec<String> support using execute/query

use conservator::{Executor, PooledConnection};
use deadpool_postgres::{Config, PoolConfig};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU32, Ordering};
use testcontainers::{Container, clients::Cli};
use testcontainers_modules::postgres::Postgres;
use tokio_postgres::types::ToSql;

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
    let db_name = format!("vector_test_{}", db_num);

    // Create database
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
    let mut test_config = Config::new();
    test_config.host = Some("127.0.0.1".to_string());
    test_config.port = Some(port);
    test_config.user = Some("postgres".to_string());
    test_config.password = Some("postgres".to_string());
    test_config.dbname = Some(db_name);
    test_config.pool = Some(PoolConfig {
        max_size: 2,
        ..Default::default()
    });

    conservator::PooledConnection::from_config(test_config).unwrap()
}

#[tokio::test]
async fn test_insert_and_query_vec_i32() {
    let pool = setup_test_db().await;
    let client = pool.get().await.unwrap();

    // Create table
    client
        .execute(
            "CREATE TABLE test_int_vec (
                id SERIAL PRIMARY KEY,
                numbers INT4[] NOT NULL
            )",
            &[],
        )
        .await
        .unwrap();

    let test_vec = vec![1_i32, 2, 3, 4, 5];

    // Insert data
    client
        .execute(
            "INSERT INTO test_int_vec (numbers) VALUES ($1)",
            &[&test_vec as &(dyn ToSql + Sync)],
        )
        .await
        .unwrap();

    // Query data back
    let rows = client
        .query("SELECT numbers FROM test_int_vec WHERE id = 1", &[])
        .await
        .unwrap();

    let retrieved_vec: Vec<i32> = rows[0].get(0);
    assert_eq!(retrieved_vec, test_vec);
}

#[tokio::test]
async fn test_vec_string() {
    let pool = setup_test_db().await;
    let client = pool.get().await.unwrap();

    // Create table
    client
        .execute(
            "CREATE TABLE test_string_vec (
                id SERIAL PRIMARY KEY,
                tags TEXT[] NOT NULL
            )",
            &[],
        )
        .await
        .unwrap();

    let test_vec = vec!["hello".to_string(), "world".to_string()];

    // Insert data
    client
        .execute(
            "INSERT INTO test_string_vec (tags) VALUES ($1)",
            &[&test_vec as &(dyn ToSql + Sync)],
        )
        .await
        .unwrap();

    // Query data back
    let rows = client
        .query("SELECT tags FROM test_string_vec WHERE id = 1", &[])
        .await
        .unwrap();

    let retrieved_vec: Vec<String> = rows[0].get(0);
    assert_eq!(retrieved_vec, test_vec);
}

#[tokio::test]
async fn test_all_integer_vec_types() {
    let pool = setup_test_db().await;
    let client = pool.get().await.unwrap();

    // Create table
    client
        .execute(
            "CREATE TABLE test_all_ints (
                id SERIAL PRIMARY KEY,
                i16_array INT2[],
                i32_array INT4[],
                i64_array INT8[]
            )",
            &[],
        )
        .await
        .unwrap();

    let i16_vec = vec![1_i16, 2, 3];
    let i32_vec = vec![100_i32, 200, 300];
    let i64_vec = vec![1000_i64, 2000, 3000];

    // Insert data
    client
        .execute(
            "INSERT INTO test_all_ints (i16_array, i32_array, i64_array) VALUES ($1, $2, $3)",
            &[
                &i16_vec as &(dyn ToSql + Sync),
                &i32_vec as &(dyn ToSql + Sync),
                &i64_vec as &(dyn ToSql + Sync),
            ],
        )
        .await
        .unwrap();

    // Query data back
    let rows = client
        .query(
            "SELECT i16_array, i32_array, i64_array FROM test_all_ints WHERE id = 1",
            &[],
        )
        .await
        .unwrap();

    let retrieved_i16: Vec<i16> = rows[0].get(0);
    let retrieved_i32: Vec<i32> = rows[0].get(1);
    let retrieved_i64: Vec<i64> = rows[0].get(2);

    assert_eq!(retrieved_i16, i16_vec);
    assert_eq!(retrieved_i32, i32_vec);
    assert_eq!(retrieved_i64, i64_vec);
}

#[tokio::test]
async fn test_float_vec_types() {
    let pool = setup_test_db().await;
    let client = pool.get().await.unwrap();

    // Create table
    client
        .execute(
            "CREATE TABLE test_floats (
                id SERIAL PRIMARY KEY,
                f32_array FLOAT4[],
                f64_array FLOAT8[]
            )",
            &[],
        )
        .await
        .unwrap();

    let f32_vec = vec![1.5_f32, 2.5, 3.5];
    let f64_vec = vec![10.5_f64, 20.5, 30.5];

    // Insert data
    client
        .execute(
            "INSERT INTO test_floats (f32_array, f64_array) VALUES ($1, $2)",
            &[
                &f32_vec as &(dyn ToSql + Sync),
                &f64_vec as &(dyn ToSql + Sync),
            ],
        )
        .await
        .unwrap();

    // Query data back
    let rows = client
        .query(
            "SELECT f32_array, f64_array FROM test_floats WHERE id = 1",
            &[],
        )
        .await
        .unwrap();

    let retrieved_f32: Vec<f32> = rows[0].get(0);
    let retrieved_f64: Vec<f64> = rows[0].get(1);

    assert_eq!(retrieved_f32, f32_vec);
    assert_eq!(retrieved_f64, f64_vec);
}

#[tokio::test]
async fn test_vec_i32_any_operator() {
    let pool = setup_test_db().await;
    let client = pool.get().await.unwrap();

    // Create table
    client
        .execute(
            "CREATE TABLE test_any (
                id SERIAL PRIMARY KEY,
                value INT4
            )",
            &[],
        )
        .await
        .unwrap();

    // Insert test data
    client
        .execute("INSERT INTO test_any (value) VALUES (1), (2), (3), (4), (5)", &[])
        .await
        .unwrap();

    // Use Vec<i32> with ANY operator
    let search_values = vec![2_i32, 4];
    let rows = client
        .query(
            "SELECT id FROM test_any WHERE value = ANY($1) ORDER BY id",
            &[&search_values as &(dyn ToSql + Sync)],
        )
        .await
        .unwrap();

    assert_eq!(rows.len(), 2);
    let id1: i32 = rows[0].get(0);
    let id2: i32 = rows[1].get(0);
    assert_eq!(id1, 2);
    assert_eq!(id2, 4);
}
