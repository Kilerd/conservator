use conservator::{DeleteBuilder, Domain, FromRow};

#[derive(Debug, Domain, FromRow)]
#[domain(table = "users")]
pub struct User {
    #[domain(primary_key)]
    pub id: i32,
    pub name: String,
    pub email: String,
}

fn main() {
    let result = DeleteBuilder::<User>::new()
        .filter(User::COLUMNS.id.eq(1))
        .build();
    
    assert_eq!(
        result.sql,
        "DELETE FROM users WHERE \"id\" = $1"
    );
    assert_eq!(result.values.len(), 1);
    
    println!("SQL: {}", result.sql);
    println!("Test passed!");
}

