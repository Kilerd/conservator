// 测试：InsertBuilder 基本用法
use conservator::{Creatable, Domain, FromRow};

#[derive(Debug, Domain, FromRow)]
#[domain(table = "users")]
pub struct User {
    #[domain(primary_key)]
    pub id: i32,
    pub name: String,
    pub email: String,
}

#[derive(Debug, Creatable)]
pub struct CreateUser {
    pub name: String,
    pub email: String,
}

fn main() {
    // 方式1：通过 Creatable::insert()
    let _builder = CreateUser {
        name: "test".to_string(),
        email: "test@example.com".to_string(),
    }
    .insert::<User>();

    // 方式2：通过 Domain::insert()
    let _builder2 = User::insert(CreateUser {
        name: "test".to_string(),
        email: "test@example.com".to_string(),
    });

    println!("InsertBuilder created successfully!");
}

