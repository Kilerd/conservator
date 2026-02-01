use std::marker::PhantomData;

use crate::builder::{IntoOrderByExpr, IntoOrderedField, Order, OrderByExpr, OrderedField};
use crate::expression::{Expression, FieldInfo, Operator};
use crate::value::{IntoValue, Value};

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

    /// 转换为不带泛型的 FieldInfo
    pub fn info(&self) -> FieldInfo {
        FieldInfo::new(self.name, self.table, self.is_primary_key)
    }

    /// 创建升序排序
    ///
    /// ```ignore
    /// User::select()
    ///     .order_by(User::COLUMNS.name.asc())
    ///     .all(&pool)
    /// ```
    pub fn asc(&self) -> OrderedField {
        OrderedField::new(self.info(), Order::Asc)
    }

    /// 创建降序排序
    ///
    /// ```ignore
    /// User::select()
    ///     .order_by(User::COLUMNS.created_at.desc())
    ///     .all(&pool)
    /// ```
    pub fn desc(&self) -> OrderedField {
        OrderedField::new(self.info(), Order::Desc)
    }
}

// Field<T> 默认升序排序
impl<T> IntoOrderedField for Field<T> {
    fn into_ordered_field(self) -> OrderedField {
        OrderedField::new(self.info(), Order::Asc)
    }
}

impl<T> IntoOrderedField for &Field<T> {
    fn into_ordered_field(self) -> OrderedField {
        OrderedField::new(self.info(), Order::Asc)
    }
}

// Field<T> 转换为 OrderByExpr（新接口）
impl<T> IntoOrderByExpr for Field<T> {
    fn into_order_by_expr(self) -> OrderByExpr {
        OrderByExpr::Field(OrderedField::new(self.info(), Order::Asc))
    }
}

impl<T> IntoOrderByExpr for &Field<T> {
    fn into_order_by_expr(self) -> OrderByExpr {
        OrderByExpr::Field(OrderedField::new(self.info(), Order::Asc))
    }
}

// 实现 From/Into 用于转换为 FieldInfo
impl<T> From<Field<T>> for FieldInfo {
    fn from(field: Field<T>) -> Self {
        field.info()
    }
}

impl<T> From<&Field<T>> for FieldInfo {
    fn from(field: &Field<T>) -> Self {
        field.info()
    }
}

// 为实现了 IntoValue 的类型提供表达式构建方法
impl<T: IntoValue> Field<T> {
    /// 创建 field = value 表达式
    ///
    /// ```ignore
    /// let expr = User::COLUMNS.id.eq(1);
    /// let result = expr.build();
    /// // result.sql = "\"id\" = $1"
    /// // result.values = [Value::I32(1)]
    /// ```
    pub fn eq(&self, value: T) -> Expression {
        Expression::comparison(self.info(), Operator::Eq, value.into_value())
    }

    /// 创建 field != value 表达式
    pub fn ne(&self, value: T) -> Expression {
        Expression::comparison(self.info(), Operator::Ne, value.into_value())
    }

    /// 创建 field > value 表达式
    pub fn gt(&self, value: T) -> Expression {
        Expression::comparison(self.info(), Operator::Gt, value.into_value())
    }

    /// 创建 field < value 表达式
    pub fn lt(&self, value: T) -> Expression {
        Expression::comparison(self.info(), Operator::Lt, value.into_value())
    }

    /// 创建 field >= value 表达式
    pub fn gte(&self, value: T) -> Expression {
        Expression::comparison(self.info(), Operator::Gte, value.into_value())
    }

    /// 创建 field <= value 表达式
    pub fn lte(&self, value: T) -> Expression {
        Expression::comparison(self.info(), Operator::Lte, value.into_value())
    }

    /// 创建 field BETWEEN low AND high 表达式
    ///
    /// ```ignore
    /// let expr = User::COLUMNS.age.between(18, 65);
    /// let result = expr.build();
    /// // result.sql = "\"age\" BETWEEN $1 AND $2"
    /// // result.values = [Value::I32(18), Value::I32(65)]
    /// ```
    pub fn between(&self, low: T, high: T) -> Expression {
        Expression::comparison_multi(
            self.info(),
            Operator::Between,
            vec![low.into_value(), high.into_value()],
        )
    }

    /// 创建 field IN (values) 表达式
    ///
    /// ```ignore
    /// let expr = User::COLUMNS.status.in_list(vec![1, 2, 3]);
    /// let result = expr.build();
    /// // result.sql = "\"status\" IN ($1, $2, $3)"
    /// // result.values = [Value::I32(1), Value::I32(2), Value::I32(3)]
    /// ```
    pub fn in_list(&self, values: Vec<T>) -> Expression {
        let values: Vec<Value> = values.into_iter().map(|v| v.into_value()).collect();
        Expression::comparison_multi(self.info(), Operator::In, values)
    }
}

// 为 Field<Option<T>> 提供 IS NULL / IS NOT NULL 方法
impl<T: IntoValue> Field<Option<T>> {
    /// 创建 field IS NULL 表达式
    ///
    /// ```ignore
    /// let expr = User::COLUMNS.email.is_null();
    /// let result = expr.build();
    /// // result.sql = "\"email\" IS NULL"
    /// // result.values = []
    /// ```
    pub fn is_null(&self) -> Expression {
        Expression::comparison_no_value(self.info(), Operator::IsNull)
    }

    /// 创建 field IS NOT NULL 表达式
    pub fn is_not_null(&self) -> Expression {
        Expression::comparison_no_value(self.info(), Operator::IsNotNull)
    }
}

// 为 Field<String> 提供 LIKE 方法
impl Field<String> {
    /// 创建 field LIKE pattern 表达式
    ///
    /// ```ignore
    /// let expr = User::COLUMNS.name.like("John%");
    /// let result = expr.build();
    /// // result.sql = "\"name\" LIKE $1"
    /// // result.values = [Value::new("John%".to_string())]
    /// ```
    pub fn like(&self, pattern: &str) -> Expression {
        Expression::comparison(self.info(), Operator::Like, Value::new(pattern.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_field_eq() {
        let field: Field<i32> = Field::new("id", "users", true);
        let expr = field.eq(42);
        let result = expr.build();
        assert_eq!(result.sql, "\"id\" = $1");
        assert_eq!(result.values.len(), 1);
        // Value 内容通过 Debug 验证
        assert!(format!("{:?}", result.values[0]).contains("42"));
    }

    #[test]
    fn test_field_like() {
        let field: Field<String> = Field::new("name", "users", false);
        let expr = field.like("John%");
        let result = expr.build();
        assert_eq!(result.sql, "\"name\" LIKE $1");
        assert_eq!(result.values.len(), 1);
        assert!(format!("{:?}", result.values[0]).contains("John%"));
    }

    #[test]
    fn test_field_in_list() {
        let field: Field<i32> = Field::new("status", "users", false);
        let expr = field.in_list(vec![1, 2, 3]);
        let result = expr.build();
        assert_eq!(result.sql, "\"status\" IN ($1, $2, $3)");
        assert_eq!(result.values.len(), 3);
    }

    #[test]
    fn test_field_between() {
        let field: Field<i32> = Field::new("age", "users", false);
        let expr = field.between(18, 65);
        let result = expr.build();
        assert_eq!(result.sql, "\"age\" BETWEEN $1 AND $2");
        assert_eq!(result.values.len(), 2);
        assert!(format!("{:?}", result.values[0]).contains("18"));
        assert!(format!("{:?}", result.values[1]).contains("65"));
    }

    #[test]
    fn test_field_is_null() {
        let field: Field<Option<String>> = Field::new("email", "users", false);
        let expr = field.is_null();
        let result = expr.build();
        assert_eq!(result.sql, "\"email\" IS NULL");
        assert!(result.values.is_empty());
    }

    #[test]
    fn test_field_is_not_null() {
        let field: Field<Option<i32>> = Field::new("age", "users", false);
        let expr = field.is_not_null();
        let result = expr.build();
        assert_eq!(result.sql, "\"age\" IS NOT NULL");
        assert!(result.values.is_empty());
    }

    #[test]
    fn test_combined_expression() {
        let id: Field<i32> = Field::new("id", "users", true);
        let name: Field<String> = Field::new("name", "users", false);
        let email: Field<Option<String>> = Field::new("email", "users", false);

        let expr = id.eq(1).and(name.like("John%")).or(email.is_null());
        let result = expr.build();
        assert_eq!(
            result.sql,
            "((\"id\" = $1 AND \"name\" LIKE $2) OR \"email\" IS NULL)"
        );
        assert_eq!(result.values.len(), 2);

        // 验证值的顺序和内容
        assert_eq!(result.values.len(), 2);
        assert!(format!("{:?}", result.values[0]).contains("1"));
        assert!(format!("{:?}", result.values[1]).contains("John%"));
    }
}
