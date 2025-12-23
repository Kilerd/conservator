use std::marker::PhantomData;

/// 表示数据库表中一个字段的元信息
/// 
/// `T` 是字段的 Rust 类型，用于类型安全的查询构建
#[derive(Debug)]
pub struct Field<T> {
    /// 字段名（对应 Rust 结构体字段名）
    pub name: &'static str,
    /// 数据库表名
    pub table: &'static str,
    /// 是否为主键
    pub is_primary_key: bool,
    _marker: PhantomData<T>,
}

// 手动实现 Clone 和 Copy，不要求 T: Clone/Copy
// 因为 PhantomData<T> 可以独立实现 Clone/Copy
impl<T> Clone for Field<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Field<T> {}

impl<T> Field<T> {
    /// 创建一个新的 Field 实例
    pub const fn new(name: &'static str, table: &'static str, is_primary_key: bool) -> Self {
        Self {
            name,
            table,
            is_primary_key,
            _marker: PhantomData,
        }
    }

    /// 返回带引号的列名，用于 SQL 查询
    pub fn quoted_name(&self) -> String {
        format!("\"{}\"", self.name)
    }

    /// 返回带表名前缀的完整列引用
    pub fn qualified_name(&self) -> String {
        format!("{}.\"{}\"", self.table, self.name)
    }
}

#[cfg(test)]
mod tests {
    use super::Field;

    #[test]
    fn field_type_test() {
        // 测试 Field 类型的基本功能
        let field: Field<i32> = Field::new("id", "users", true);

        assert_eq!(field.name, "id");
        assert_eq!(field.table, "users");
        assert!(field.is_primary_key);
        assert_eq!(field.quoted_name(), "\"id\"");
        assert_eq!(field.qualified_name(), "users.\"id\"");
    }
}

