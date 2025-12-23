# Conservator ORM

Conservator ORM is based on sqlx, currently it only supports PostgreSQL.

## Quick Start

### Define Domain Entity

```rust
#[derive(Debug, Domain, FromRow)]
#[domain(table = "users")]
pub struct User {
    #[domain(primary_key)]
    pub id: i32,
    pub name: String,
    pub email: String,
}
```

The struct derived `Domain` auto-generates methods like:
- `find_by_pk` - return optional entity
- `fetch_one_by_pk` - return entity or raise
- `fetch_all` - return all entities
- `delete_by_pk` - delete by primary key

### Define Creatable

```rust
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
    .order_by(User::COLUMNS.id, Order::Asc)
    .limit(10)
    .all(db)
    .await?;

// Find one
let user = User::select()
    .filter(User::COLUMNS.id.eq(1))
    .one(db)
    .await?;

// Return custom type with .returning()
#[derive(Debug, Domain, FromRow)]
#[domain(table = "users")]
pub struct UserSummary {
    #[domain(primary_key)]
    pub id: i32,
    pub name: String,
}

let summaries: Vec<UserSummary> = User::select()
    .returning::<UserSummary>()
    .filter(User::COLUMNS.active.eq(true))
    .all(db)
    .await?;
```

**Note:** Use `.returning::<T>()` to return a different type than the Domain. The returned type must implement `Domain` and `FromRow`, and its `COLUMN_NAMES` will be used in the SELECT clause.

### INSERT

```rust
// Single insert - returning primary key
let pk = CreateUser { name: "test".into(), email: "a@b.com".into() }
    .insert::<User>()
    .returning_pk(db)
    .await?;

// Single insert - returning entity
let user = CreateUser { name: "test".into(), email: "a@b.com".into() }
    .insert::<User>()
    .returning_entity(db)
    .await?;

// Batch insert
let pks = User::insert_many(vec![
    CreateUser { name: "a".into(), email: "a@b.com".into() },
    CreateUser { name: "b".into(), email: "b@b.com".into() },
])
.returning_pk(db)
.await?;  // Vec<i32>
```

### UPDATE

```rust
// Type-safe update - must have both SET and FILTER
let rows = User::update_query()
    .set(User::COLUMNS.name, "new_name".to_string())
    .set(User::COLUMNS.email, "new@email.com".to_string())
    .filter(User::COLUMNS.id.eq(1))
    .execute(db)
    .await?;
```

**Note:** `UpdateBuilder` uses type-state pattern to ensure you must call both `.set()` and `.filter()` before `.execute()`. This prevents accidental updates without conditions.

### DELETE

```rust
// Type-safe delete - must have FILTER
let rows = User::delete()
    .filter(User::COLUMNS.id.eq(1))
    .execute(db)
    .await?;
```

**Note:** `DeleteBuilder` uses type-state pattern to ensure you must call `.filter()` before `.execute()`. This prevents accidental deletion of all rows.

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
```

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

**Note:** Rather than sqlx's `$1`, we use param `:email` in SQL. This can be used in native SQL execution tools (like IDEA) without modification.
