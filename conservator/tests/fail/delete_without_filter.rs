// 测试：没有设置 filter 的 DeleteBuilder 不能调用 build()
// 这应该编译失败，因为 build() 只在 DeleteBuilder<T, true> 上实现
use conservator::{Domain, FromRow};

#[derive(Debug, Domain, FromRow)]
#[domain(table = "users")]
pub struct User {
    #[domain(primary_key)]
    pub id: i32,
    pub name: String,
    pub email: String,
}

fn main() {
    // 没有设置 filter，尝试调用 build() 应该编译失败
    let _result = User::delete().build();
}

