// 测试：只有 filter 没有 set 的 UpdateBuilder 不能调用 build()
// 这应该编译失败，因为 build() 只在 UpdateBuilder<T, true, true> 上实现
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
    // 只有 filter，没有 set，尝试调用 build() 应该编译失败
    let _result = User::update_query()
        .filter(User::COLUMNS.id.eq(1))
        .build();
}

