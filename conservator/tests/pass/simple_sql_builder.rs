use conservator::{Domain, SelectBuilder};

#[derive(Debug, Domain)]
#[domain(table = "users")]
pub struct User {
    #[domain(primary_key)]
    pub id: i32,
    pub name: String,
    pub email: String,
}

fn main() {
    let result = SelectBuilder::<User>::new()
        .filter(User::COLUMNS.id.eq(1))
        .build();
    
    assert_eq!(
        result.sql,
        "SELECT \"id\", \"name\", \"email\" FROM users WHERE \"id\" = $1"
    );
    assert_eq!(result.values.len(), 1);
    
    println!("SQL: {}", result.sql);
    println!("Test passed!");
}
