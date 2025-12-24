use conservator::Domain;
use conservator::SqlType;
use std::error::Error;
use tokio_postgres::types::{private::BytesMut, IsNull, Type};

#[derive(Debug, Clone)]
pub struct CustomType {
    pub id: i32,
    pub name: String,
    pub email: String,
}


impl SqlType for CustomType {
    fn to_sql_value(&self, ty: &Type, out: &mut BytesMut) -> Result<IsNull, Box<dyn Error + Sync + Send>> {
        out.extend_from_slice(self.id.to_string().as_bytes());
        Ok(IsNull::No)
    }

    fn from_sql_value(ty: &Type, raw: &[u8]) -> Result<Self, Box<dyn Error + Sync + Send>> {
        Ok(CustomType {
            id: 1,
            name: "".to_string(),
            email: "".to_string(),
        })
    }
    
    fn accepts(ty: &Type) -> bool {
        matches!(*ty, Type::NUMERIC | Type::TEXT | Type::VARCHAR)
    }

    fn from_sql_null_value(_ty: &Type) -> Result<Self, Box<dyn Error + Sync + Send>> {
        Err("unexpected NULL value".into())
    }
}

// IntoValue 现在通过泛型自动实现：impl<T: SqlType + 'static> IntoValue for T

#[derive(Debug, Domain)]
#[domain(table = "users")]
pub struct User {
    #[domain(primary_key)]
    pub id: i32,
    pub name: CustomType,
    pub email: String,
    pub custom_type: CustomType,
}

fn main() {
    println!("Custom type test passed!");
}