use conservator::{Domain, Field, FromRow};

#[derive(Debug, Domain, FromRow)]
#[domain(table = "users")]
pub struct User {
    #[domain(primary_key)]
    pub id: i32,
    pub name: String,
    pub email: String,
}

fn main() {
    // 测试 COLUMNS 常量存在且可访问
    let id_field: Field<i32> = User::COLUMNS.id;
    let name_field: Field<String> = User::COLUMNS.name;
    let email_field: Field<String> = User::COLUMNS.email;
    
    // 测试字段元信息
    assert_eq!(id_field.name, "id");
    assert_eq!(id_field.table, "users");
    assert!(id_field.is_primary_key);
    
    assert_eq!(name_field.name, "name");
    assert_eq!(name_field.table, "users");
    assert!(!name_field.is_primary_key);
    
    assert_eq!(email_field.name, "email");
    assert_eq!(email_field.table, "users");
    assert!(!email_field.is_primary_key);
    
    // 测试辅助方法
    assert_eq!(id_field.quoted_name(), "\"id\"");
    assert_eq!(id_field.qualified_name(), "users.\"id\"");
    
    println!("All tests passed!");
}

