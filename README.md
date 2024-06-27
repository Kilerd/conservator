# Conservator ORM

Conservator ORM is based on sqlx, currently it only support postgres

```rust
#[derive(Debug, Deserialize, Serialize, Domain, FromRow)]
#[domain(table = "users")]
pub struct UserDomain {
    #[domain(primary_key)]
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub password: String,
    pub role: UserRole,
    pub create_at: DateTime<Utc>,
    pub last_login_at: DateTime<Utc>,
}
```
the struct derived `Domain` would auto generate methods like:
- `find_by_id` return optional entity
- `fetch_one_by_id` return entity or raise
- `fetch_all` return all entities
- `create` passing the `Createable` to insert into table

```rust
#[derive(Debug, Deserialize, Serialize, Creatable)]
pub struct NewUser {
    pub username: String,
    pub email: String,
    pub password: String,
}
```

`Createable` means it can be executed by magic ORM, using `UserDomain::create(NewUser{...})` to create a new user into
user table.


`#[sql]` aslo provide some convinent way to write customized sql query
```rust
use conservator::sql;

impl UserService {

    #[sql(find)]
    pub async fn find_user<E>(email: &str, executor: E) -> Result<Option<UserEntity>, Error> {
        "select * from users where email = :email"
    }
}
```
notice that, rather than sqlx's `$1`, we use param `:email` in sql, it can be used in native sql execution tools as well without any modification, like IDEA.
