// 测试：设置了 filter 的 UpdateBuilder 可以正常 build 和使用
use conservator::Domain;

#[derive(Debug, Domain)]
#[domain(table = "users")]
pub struct User {
    #[domain(primary_key)]
    pub id: i32,
    pub name: String,
    pub email: String,
}

fn main() {
    // 单字段更新
    let result = User::update()
        .set(User::COLUMNS.name, "new_name".to_string())
        .filter(User::COLUMNS.id.eq(1))
        .build();
    
    assert_eq!(result.sql, "UPDATE users SET \"name\" = $1 WHERE \"id\" = $2");
    assert_eq!(result.values.len(), 2);
    
    // 多字段更新
    let result2 = User::update()
        .set(User::COLUMNS.name, "new_name".to_string())
        .set(User::COLUMNS.email, "new@email.com".to_string())
        .filter(User::COLUMNS.id.eq(1))
        .build();
    
    assert_eq!(
        result2.sql,
        "UPDATE users SET \"name\" = $1, \"email\" = $2 WHERE \"id\" = $3"
    );
    assert_eq!(result2.values.len(), 3);
    
    // 复杂条件
    let result3 = User::update()
        .set(User::COLUMNS.name, "new_name".to_string())
        .filter(User::COLUMNS.id.eq(1) & User::COLUMNS.email.like("%@old.com"))
        .build();
    
    assert!(result3.sql.contains("UPDATE users SET"));
    assert!(result3.sql.contains("WHERE"));
    assert_eq!(result3.values.len(), 3);
    
    println!("All update with filter tests passed!");
}

