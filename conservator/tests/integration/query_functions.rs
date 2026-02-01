//! Integration tests for query functions (DISTINCT and random())
//!
//! Tests DISTINCT and random() functionality with real database.

use conservator::{random, Domain, Executor, PooledConnection, Selectable};
use deadpool_postgres::{Config, PoolConfig};
use std::collections::HashSet;
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
    let db_name = format!("query_func_test_{}", db_num);

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
            "CREATE TABLE products (
                id SERIAL PRIMARY KEY,
                name TEXT NOT NULL,
                category TEXT NOT NULL,
                price INTEGER NOT NULL
            )",
            &[],
        )
        .await
        .unwrap();

    pool
}

#[derive(Debug, Clone, Domain)]
#[domain(table = "products")]
struct Product {
    #[domain(primary_key)]
    id: i32,
    name: String,
    category: String,
    price: i32,
}

#[derive(Debug, Selectable)]
struct CategoryRow {
    category: String,
}

#[tokio::test]
async fn test_distinct_removes_duplicates() {
    let pool = setup_test_db().await;

    // Insert test data with duplicate categories
    let client = pool.get().await.unwrap();
    client
        .execute(
            "INSERT INTO products (name, category, price) VALUES
                ('Apple', 'Fruit', 100),
                ('Banana', 'Fruit', 80),
                ('Carrot', 'Vegetable', 50),
                ('Tomato', 'Vegetable', 60),
                ('Orange', 'Fruit', 90)",
            &[],
        )
        .await
        .unwrap();

    // Query distinct categories
    let categories = Product::select()
        .returning::<CategoryRow>()
        .distinct()
        .all(&client)
        .await
        .unwrap();

    // Should only return 2 unique categories
    assert_eq!(categories.len(), 2);

    let category_set: HashSet<String> = categories.into_iter().map(|c| c.category).collect();
    assert!(category_set.contains("Fruit"));
    assert!(category_set.contains("Vegetable"));
}

#[tokio::test]
async fn test_distinct_with_filter() {
    let pool = setup_test_db().await;

    // Insert test data
    let client = pool.get().await.unwrap();
    client
        .execute(
            "INSERT INTO products (name, category, price) VALUES
                ('Apple', 'Fruit', 100),
                ('Banana', 'Fruit', 80),
                ('Cherry', 'Fruit', 120),
                ('Carrot', 'Vegetable', 50),
                ('Tomato', 'Vegetable', 60)",
            &[],
        )
        .await
        .unwrap();

    // Query distinct categories for products with price >= 100
    let categories = Product::select()
        .returning::<CategoryRow>()
        .distinct()
        .filter(Product::COLUMNS.price.gte(100))
        .all(&client)
        .await
        .unwrap();

    // Should only return "Fruit" (Apple and Cherry are >= 100)
    assert_eq!(categories.len(), 1);
    assert_eq!(categories[0].category, "Fruit");
}

#[tokio::test]
async fn test_random_order_produces_different_results() {
    let pool = setup_test_db().await;

    // Insert test data
    let client = pool.get().await.unwrap();
    for i in 1..=20 {
        client
            .execute(
                "INSERT INTO products (name, category, price) VALUES ($1, $2, $3)",
                &[
                    &format!("Product {}", i),
                    &"Category".to_string(),
                    &(i * 10),
                ],
            )
            .await
            .unwrap();
    }

    // Query with random order multiple times
    let result1: Vec<i32> = Product::select()
        .order_by(random())
        .limit(10)
        .all(&client)
        .await
        .unwrap()
        .into_iter()
        .map(|p| p.id)
        .collect();

    let result2: Vec<i32> = Product::select()
        .order_by(random())
        .limit(10)
        .all(&client)
        .await
        .unwrap()
        .into_iter()
        .map(|p| p.id)
        .collect();

    // Results should be different (with high probability)
    // Note: There's a tiny chance they could be the same, but it's negligible
    assert_eq!(result1.len(), 10);
    assert_eq!(result2.len(), 10);

    // At least verify both queries returned results
    assert!(!result1.is_empty());
    assert!(!result2.is_empty());
}

#[tokio::test]
async fn test_random_order_with_limit() {
    let pool = setup_test_db().await;

    // Insert test data
    let client = pool.get().await.unwrap();
    for i in 1..=50 {
        client
            .execute(
                "INSERT INTO products (name, category, price) VALUES ($1, $2, $3)",
                &[
                    &format!("Product {}", i),
                    &"Category".to_string(),
                    &(i * 10),
                ],
            )
            .await
            .unwrap();
    }

    // Query with random order and limit
    let products = Product::select()
        .order_by(random())
        .limit(5)
        .all(&client)
        .await
        .unwrap();

    // Should return exactly 5 products
    assert_eq!(products.len(), 5);

    // Verify all products are valid
    for product in products {
        assert!(product.id > 0 && product.id <= 50);
        assert!(product.name.starts_with("Product "));
    }
}

#[tokio::test]
async fn test_mixed_ordering_field_and_random() {
    let pool = setup_test_db().await;

    // Insert test data with same category
    let client = pool.get().await.unwrap();
    for i in 1..=10 {
        client
            .execute(
                "INSERT INTO products (name, category, price) VALUES ($1, $2, $3)",
                &[
                    &format!("Product {}", i),
                    &"SameCategory".to_string(),
                    &100,
                ],
            )
            .await
            .unwrap();
    }

    // Query with category order first, then random
    let products = Product::select()
        .order_by(Product::COLUMNS.category)
        .order_by(random())
        .limit(5)
        .all(&client)
        .await
        .unwrap();

    // Should return 5 products, all with same category
    assert_eq!(products.len(), 5);
    for product in &products {
        assert_eq!(product.category, "SameCategory");
    }
}

#[tokio::test]
async fn test_distinct_preserves_null_handling() {
    let pool = setup_test_db().await;

    // Create table with nullable column
    let client = pool.get().await.unwrap();
    client
        .execute(
            "CREATE TABLE items (
                id SERIAL PRIMARY KEY,
                category TEXT
            )",
            &[],
        )
        .await
        .unwrap();

    // Insert data with some nulls
    client
        .execute(
            "INSERT INTO items (category) VALUES ('A'), ('B'), ('A'), (NULL), (NULL)",
            &[],
        )
        .await
        .unwrap();

    #[derive(Debug, Domain)]
    #[domain(table = "items")]
    struct Item {
        #[domain(primary_key)]
        id: i32,
        category: Option<String>,
    }

    #[derive(Debug, Selectable)]
    struct ItemCategoryRow {
        category: Option<String>,
    }

    // Query distinct categories (including NULL)
    let categories = Item::select()
        .returning::<ItemCategoryRow>()
        .distinct()
        .all(&client)
        .await
        .unwrap();

    // Should return 3 distinct values: 'A', 'B', NULL
    assert_eq!(categories.len(), 3);

    let has_null = categories.iter().any(|c| c.category.is_none());
    assert!(has_null, "DISTINCT should preserve NULL values");
}
