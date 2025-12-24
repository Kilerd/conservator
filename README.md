# Conservator ORM

A lightweight, type-safe ORM for PostgreSQL built on `tokio-postgres`.

## Features

- 🚀 Built on `tokio-postgres` for high performance
- 🔒 Type-safe query builders with compile-time guarantees
- 📦 Connection pooling via `deadpool-postgres`
- 🎯 Derive macros for minimal boilerplate
- 💡 Active Record and Query Builder patterns
- 🔧 Extensible type system via `SqlType` trait

## Quick Start

### Installation

```toml
[dependencies]
conservator = "0.2"
tokio = { version = "1", features = ["full"] }
```

### Connection Pool

```rust
use conservator::PooledConnection;

// Create connection pool from URL
let pool = PooledConnection::from_url("postgres://user:pass@localhost:5432/dbname")?;

// Get a connection
let conn = pool.get().await?;

// Or use pool directly (acquires connection per query)
let users = User::select().all(&pool).await?;
```

### Define Domain Entity

```rust
use conservator::Domain;

#[derive(Debug, Domain)]
#[domain(table = "users")]
pub struct User {
    #[domain(primary_key)]
    pub id: i32,
    pub name: String,
    pub email: String,
}
```

The `#[derive(Domain)]` macro automatically generates:
- `Selectable` trait implementation (with `COLUMN_NAMES` and `from_row`)
- `Domain` trait implementation with CRUD methods

### Auto-generated Methods

- `find_by_pk` - return optional entity
- `fetch_one_by_pk` - return entity or raise
- `fetch_all` - return all entities
- `delete_by_pk` - delete by primary key
- `update` - save entity changes to database (Active Record style)

### Define Creatable

```rust
use conservator::Creatable;

#[derive(Debug, Creatable)]
pub struct CreateUser {
    pub name: String,
    pub email: String,
}
```

## Query Builders

Conservator provides type-safe query builders for SELECT, INSERT, UPDATE, and DELETE operations.

### SELECT

```rust
// Simple select
let users = User::select()
    .filter(User::COLUMNS.name.like("John%"))
    .order_by(User::COLUMNS.id)  // Default ascending
    .limit(10)
    .all(&pool)
    .await?;

// Find one
let user = User::select()
    .filter(User::COLUMNS.id.eq(1))
    .one(&pool)
    .await?;

// Order by with explicit direction
let users = User::select()
    .order_by(User::COLUMNS.created_at.desc())  // Explicit descending
    .order_by(User::COLUMNS.name)               // Default ascending
    .all(&pool)
    .await?;
```

#### Custom Return Type with `Selectable`

Use `#[derive(Selectable)]` to define lightweight projection types:

```rust
use conservator::Selectable;

#[derive(Debug, Selectable)]
pub struct UserSummary {
    pub id: i32,
    pub name: String,
}

// Use .returning() to switch return type
let summaries: Vec<UserSummary> = User::select()
    .returning::<UserSummary>()
    .filter(User::COLUMNS.active.eq(true))
    .all(&pool)
    .await?;
```

### INSERT

```rust
// Single insert - returning primary key
let pk = CreateUser { name: "test".into(), email: "a@b.com".into() }
    .insert::<User>()
    .returning_pk(&pool)
    .await?;

// Single insert - returning entity
let user = CreateUser { name: "test".into(), email: "a@b.com".into() }
    .insert::<User>()
    .returning_entity(&pool)
    .await?;

// Batch insert
let pks = User::insert_many(vec![
    CreateUser { name: "a".into(), email: "a@b.com".into() },
    CreateUser { name: "b".into(), email: "b@b.com".into() },
])
.returning_pk(&pool)
.await?;  // Vec<i32>
```

### UPDATE

#### Query Builder Style

```rust
// Type-safe update - must have both SET and FILTER
let rows = User::update()
    .set(User::COLUMNS.name, "new_name".to_string())
    .set(User::COLUMNS.email, "new@email.com".to_string())
    .filter(User::COLUMNS.id.eq(1))
    .execute(&pool)
    .await?;
```

**Note:** `UpdateBuilder` uses type-state pattern to ensure you must call both `.set()` and `.filter()` before `.execute()`.

#### Active Record Style

```rust
// Fetch entity, modify, then save
let mut user = User::fetch_one_by_pk(&1, &pool).await?;
user.name = "New Name".to_string();
user.email = "new@email.com".to_string();
user.update(&pool).await?;  // Updates all non-PK fields
```

### DELETE

```rust
// Type-safe delete - must have FILTER
let rows = User::delete()
    .filter(User::COLUMNS.id.eq(1))
    .execute(&pool)
    .await?;
```

**Note:** `DeleteBuilder` uses type-state pattern to ensure you must call `.filter()` before `.execute()`.

## Transactions

```rust
let pool = PooledConnection::from_url("postgres://...")?;
let mut conn = pool.get().await?;

// Start transaction
let tx = conn.begin().await?;

// Execute operations within transaction
let pk = CreateUser { name: "test".into(), email: "a@b.com".into() }
    .insert::<User>()
    .returning_pk(&tx)
    .await?;

// Commit (or rollback on drop)
tx.commit().await?;
```

## Expression System

Build complex WHERE conditions with type-safe expressions:

```rust
// Comparison operators
User::COLUMNS.id.eq(1)           // id = $1
User::COLUMNS.id.ne(1)           // id != $1
User::COLUMNS.id.gt(10)          // id > $1
User::COLUMNS.id.gte(10)         // id >= $1
User::COLUMNS.id.lt(100)         // id < $1
User::COLUMNS.id.lte(100)        // id <= $1

// String operations
User::COLUMNS.name.like("John%") // name LIKE $1

// Range operations
User::COLUMNS.id.between(1, 100) // id BETWEEN $1 AND $2
User::COLUMNS.id.in_list(vec![1, 2, 3]) // id IN ($1, $2, $3)

// NULL checks (only for Option<T> fields)
User::COLUMNS.deleted_at.is_null()     // deleted_at IS NULL
User::COLUMNS.deleted_at.is_not_null() // deleted_at IS NOT NULL

// Logical operators
let expr = User::COLUMNS.id.eq(1) & User::COLUMNS.name.like("John%");  // AND
let expr = User::COLUMNS.id.eq(1) | User::COLUMNS.id.eq(2);            // OR

// Ordering
User::COLUMNS.id              // Default ascending (ASC)
User::COLUMNS.id.asc()        // Explicit ascending
User::COLUMNS.id.desc()       // Descending
```

## Supported Types

| Rust Type | PostgreSQL Type |
|-----------|-----------------|
| `i16`, `i32`, `i64` | `SMALLINT`, `INTEGER`, `BIGINT` |
| `f32`, `f64` | `REAL`, `DOUBLE PRECISION` |
| `bool` | `BOOLEAN` |
| `String` | `TEXT`, `VARCHAR` |
| `uuid::Uuid` | `UUID` |
| `chrono::DateTime<Utc>` | `TIMESTAMPTZ` |
| `rust_decimal::Decimal` | `NUMERIC` |
| `serde_json::Value` | `JSONB` |
| `Option<T>` | Nullable columns |

## Custom Types with `SqlType`

Extend conservator to support custom PostgreSQL types:

```rust
use conservator::SqlType;
use tokio_postgres::types::{Type, IsNull, private::BytesMut};

#[derive(Debug, Clone)]
pub struct MyCustomType { /* ... */ }

impl SqlType for MyCustomType {
    fn to_sql_value(&self, ty: &Type, out: &mut BytesMut) -> Result<IsNull, Box<dyn std::error::Error + Sync + Send>> {
        // Serialize to PostgreSQL format
        Ok(IsNull::No)
    }

    fn from_sql_value(ty: &Type, raw: &[u8]) -> Result<Self, Box<dyn std::error::Error + Sync + Send>> {
        // Deserialize from PostgreSQL format
        Ok(MyCustomType { /* ... */ })
    }

    fn accepts(ty: &Type) -> bool {
        // Check if this type is accepted
        true
    }
}

// Now use in your entities
#[derive(Debug, Domain)]
#[domain(table = "items")]
pub struct Item {
    #[domain(primary_key)]
    pub id: i32,
    pub custom_field: MyCustomType,
}
```

## Traits Overview

| Trait | Description | Derive Macro |
|-------|-------------|--------------|
| `Selectable` | Lightweight trait with `COLUMN_NAMES` and `from_row` | `#[derive(Selectable)]` |
| `Domain` | Full CRUD operations, inherits `Selectable` | `#[derive(Domain)]` |
| `Creatable` | For INSERT data structures | `#[derive(Creatable)]` |
| `SqlType` | Custom type support for PostgreSQL | Manual impl |
| `Executor` | Database execution abstraction | Auto-implemented |

## Custom SQL with `#[sql]`

```rust
use conservator::sql;

impl UserService {
    #[sql(find)]
    pub async fn find_user(email: &str) -> Option<User> {
        "select * from users where email = :email"
    }
}
```

**Note:** Use named parameters `:email` instead of `$1`. This allows the SQL to be used directly in database tools.

## Migration (Optional)

Enable the `migrate` feature to use sqlx migrations:

```toml
[dependencies]
conservator = { version = "0.2", features = ["migrate"] }
```

```rust
use conservator::migrate;

// Run migrations
migrate!("./migrations").run(&pool).await?;
```

## License

MIT
