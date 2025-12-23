// 测试：Selectable trait 用于 returning() 方法
use conservator::{Domain, Selectable};

// 完整的 Domain 实体
#[derive(Debug, Domain)]
#[domain(table = "users")]
pub struct User {
    #[domain(primary_key)]
    pub id: i32,
    pub name: String,
    pub email: String,
}

// 轻量级投影类型，只需要 Selectable
#[derive(Debug, Selectable)]
pub struct UserSummary {
    pub id: i32,
    pub name: String,
}

fn main() {
    // 测试 Selectable trait 的 COLUMN_NAMES
    assert_eq!(User::COLUMN_NAMES, &["id", "name", "email"]);
    assert_eq!(UserSummary::COLUMN_NAMES, &["id", "name"]);
    
    // 测试使用 returning() 切换返回类型
    let result = User::select()
        .returning::<UserSummary>()
        .filter(User::COLUMNS.id.eq(1))
        .build();
    
    // 验证 SQL 使用了 UserSummary 的列名
    assert_eq!(
        result.sql,
        "SELECT \"id\", \"name\" FROM users WHERE \"id\" = $1"
    );
    
    println!("Selectable test passed!");
}

