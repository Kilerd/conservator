//! Shared facilities for benchmarks

use conservator::{Creatable, Domain, Executor, PooledConnection};
use deadpool_postgres::{Config, PoolConfig};
use std::sync::{
    OnceLock,
    atomic::{AtomicU32, Ordering},
};
use testcontainers::{Container, clients::Cli};
use testcontainers_modules::postgres::Postgres;

// Shared Docker client and container
static DOCKER: OnceLock<Cli> = OnceLock::new();
static POSTGRES_CONTAINER: OnceLock<Container<'static, Postgres>> = OnceLock::new();
static DB_COUNTER: AtomicU32 = AtomicU32::new(0);

pub fn docker() -> &'static Cli {
    DOCKER.get_or_init(Cli::default)
}

pub fn postgres_container() -> &'static Container<'static, Postgres> {
    POSTGRES_CONTAINER.get_or_init(|| {
        let docker = docker();
        docker.run(Postgres::default())
    })
}

fn unique_db_name() -> String {
    let count = DB_COUNTER.fetch_add(1, Ordering::SeqCst);
    format!("bench_db_{}", count)
}

pub async fn create_test_pool() -> PooledConnection {
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
        max_size: 4,
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
        max_size: 4,
        ..Default::default()
    });

    let pool = PooledConnection::from_config(config).unwrap();

    let conn = pool.get().await.unwrap();
    conn.client()
        .batch_execute(
            "CREATE TABLE users (
                id SERIAL PRIMARY KEY,
                name TEXT NOT NULL,
                email TEXT NOT NULL,
                age INTEGER NOT NULL
            )",
        )
        .await
        .unwrap();

    pool
}

#[derive(Debug, Domain)]
#[domain(table = "users")]
pub struct User {
    #[domain(primary_key)]
    pub id: i32,
    pub name: String,
    pub email: String,
    pub age: i32,
}

#[derive(Debug, Creatable)]
pub struct CreateUser {
    pub name: String,
    pub email: String,
    pub age: i32,
}

impl CreateUser {
    pub fn sample(index: i32) -> Self {
        Self {
            name: format!("User{}", index),
            email: format!("user{}@example.com", index),
            age: 20 + (index % 50),
        }
    }
}

/// Populate database with sample data
#[allow(dead_code)]
pub async fn populate_sample_data(pool: &PooledConnection, count: usize) {
    for i in 1..=count {
        CreateUser::sample(i as i32)
            .insert::<User>()
            .returning_pk(pool)
            .await
            .unwrap();
    }
}
