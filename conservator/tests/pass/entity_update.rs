// 测试：Domain::save() 方法的 Active Record 风格更新
use conservator::Domain;

#[derive(Debug, Domain)]
#[domain(table = "users")]
pub struct User {
    #[domain(primary_key)]
    pub id: i32,
    pub name: String,
    pub email: String,
}

// 验证 save 方法可以通过 &self 调用（编译时检查）
#[allow(dead_code)]
async fn test_save_signature(user: &User, pool: &conservator::PooledConnection) {
    // 这行代码验证 save 方法使用 &self 调用
    let _ = user.save(pool).await;
}

fn main() {
    println!("Entity save test passed!");
}

