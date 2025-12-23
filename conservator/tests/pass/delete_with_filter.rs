// 测试：设置了 filter 的 DeleteBuilder 可以正常 build 和使用
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
    
    // 设置了 filter，应该可以调用 build()
    let result = User::delete()
        .filter(User::COLUMNS.id.eq(1))
        .build();
    
    assert_eq!(result.sql, "DELETE FROM users WHERE \"id\" = $1");
    assert_eq!(result.values.len(), 1);
    
    // 多条件 filter
    let result2 = User::delete()
        .filter(User::COLUMNS.id.eq(1) & User::COLUMNS.name.eq("test".to_string()))
        .build();
    
    assert!(result2.sql.contains("DELETE FROM users WHERE"));
    assert_eq!(result2.values.len(), 2);
    
    println!("All delete with filter tests passed!");
}

