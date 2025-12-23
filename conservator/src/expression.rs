/// 字段元信息（不带泛型，用于存储在 Expression 中）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FieldInfo {
    /// 字段名
    pub name: &'static str,
    /// 表名
    pub table: &'static str,
    /// 是否为主键
    pub is_primary_key: bool,
}

impl FieldInfo {
    /// 创建新的 FieldInfo
    pub const fn new(name: &'static str, table: &'static str, is_primary_key: bool) -> Self {
        Self {
            name,
            table,
            is_primary_key,
        }
    }

    /// 返回带引号的列名
    pub fn quoted_name(&self) -> String {
        format!("\"{}\"", self.name)
    }

    /// 返回带表名前缀的完整列引用
    pub fn qualified_name(&self) -> String {
        format!("{}.\"{}\"", self.table, self.name)
    }
}

/// SQL 操作符
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operator {
    /// =
    Eq,
    /// !=
    Ne,
    /// >
    Gt,
    /// <
    Lt,
    /// >=
    Gte,
    /// <=
    Lte,
    /// LIKE
    Like,
    /// IN
    In,
    /// IS NULL
    IsNull,
    /// IS NOT NULL
    IsNotNull,
    /// BETWEEN
    Between,
}

impl Operator {
    /// 返回操作符的 SQL 表示
    pub fn to_sql(&self) -> &'static str {
        match self {
            Operator::Eq => "=",
            Operator::Ne => "!=",
            Operator::Gt => ">",
            Operator::Lt => "<",
            Operator::Gte => ">=",
            Operator::Lte => "<=",
            Operator::Like => "LIKE",
            Operator::In => "IN",
            Operator::IsNull => "IS NULL",
            Operator::IsNotNull => "IS NOT NULL",
            Operator::Between => "BETWEEN",
        }
    }
}

/// 存储 SQL 参数值的枚举
/// 
/// 支持常见的数据库类型
#[derive(Debug, Clone)]
pub enum Value {
    Bool(bool),
    I16(i16),
    I32(i32),
    I64(i64),
    F32(f32),
    F64(f64),
    String(String),
    Bytes(Vec<u8>),
    /// 用于扩展其他类型
    None,
}

/// 将 Rust 类型转换为 Value 的 trait
pub trait IntoValue {
    fn into_value(self) -> Value;
}

impl IntoValue for bool {
    fn into_value(self) -> Value {
        Value::Bool(self)
    }
}

impl IntoValue for i16 {
    fn into_value(self) -> Value {
        Value::I16(self)
    }
}

impl IntoValue for i32 {
    fn into_value(self) -> Value {
        Value::I32(self)
    }
}

impl IntoValue for i64 {
    fn into_value(self) -> Value {
        Value::I64(self)
    }
}

impl IntoValue for f32 {
    fn into_value(self) -> Value {
        Value::F32(self)
    }
}

impl IntoValue for f64 {
    fn into_value(self) -> Value {
        Value::F64(self)
    }
}

impl IntoValue for String {
    fn into_value(self) -> Value {
        Value::String(self)
    }
}

impl IntoValue for &str {
    fn into_value(self) -> Value {
        Value::String(self.to_string())
    }
}

impl<T: IntoValue + Clone> IntoValue for &T {
    fn into_value(self) -> Value {
        self.clone().into_value()
    }
}

/// SQL 表达式
/// 
/// 包含 SQL 片段和绑定的参数值
#[derive(Debug, Clone)]
pub enum Expression {
    /// 比较表达式: field op value
    Comparison {
        /// 字段元信息
        field: FieldInfo,
        /// 操作符
        operator: Operator,
        /// 绑定的参数值
        values: Vec<Value>,
    },
    /// AND 组合表达式
    And(Box<Expression>, Box<Expression>),
    /// OR 组合表达式
    Or(Box<Expression>, Box<Expression>),
}

/// 表达式生成的 SQL 结果
#[derive(Debug, Clone)]
pub struct SqlResult {
    /// SQL 字符串（带 $1, $2 等占位符）
    pub sql: String,
    /// 绑定的参数值（按顺序）
    pub values: Vec<Value>,
}

impl Expression {
    /// 创建一个带单个值的比较表达式
    pub fn comparison(field: FieldInfo, operator: Operator, value: Value) -> Self {
        Expression::Comparison {
            field,
            operator,
            values: vec![value],
        }
    }

    /// 创建一个无值的比较表达式（用于 IS NULL 等）
    pub fn comparison_no_value(field: FieldInfo, operator: Operator) -> Self {
        Expression::Comparison {
            field,
            operator,
            values: vec![],
        }
    }

    /// 创建一个带多个值的比较表达式（用于 IN, BETWEEN）
    pub fn comparison_multi(field: FieldInfo, operator: Operator, values: Vec<Value>) -> Self {
        Expression::Comparison {
            field,
            operator,
            values,
        }
    }

    /// 获取表达式中涉及的所有字段信息
    pub fn fields(&self) -> Vec<FieldInfo> {
        match self {
            Expression::Comparison { field, .. } => vec![*field],
            Expression::And(left, right) | Expression::Or(left, right) => {
                let mut fields = left.fields();
                fields.extend(right.fields());
                fields
            }
        }
    }

    /// 用 AND 组合两个表达式
    pub fn and(self, other: Expression) -> Expression {
        Expression::And(Box::new(self), Box::new(other))
    }

    /// 用 OR 组合两个表达式
    pub fn or(self, other: Expression) -> Expression {
        Expression::Or(Box::new(self), Box::new(other))
    }

    /// 生成完整的 SQL 结果
    /// 
    /// 返回包含 SQL 字符串和参数值的 SqlResult
    pub fn build(self) -> SqlResult {
        let (sql, values, _) = self.build_internal(1);
        SqlResult { sql, values }
    }

    /// 内部构建方法（使用带引号的字段名）
    /// 
    /// 返回 (sql, values, next_param_index)
    fn build_internal(self, start_param: usize) -> (String, Vec<Value>, usize) {
        self.build_internal_with_qualifier(start_param, false)
    }

    /// 内部构建方法
    /// 
    /// `use_qualified` 为 true 时使用 table."column" 格式
    fn build_internal_with_qualifier(
        self,
        start_param: usize,
        use_qualified: bool,
    ) -> (String, Vec<Value>, usize) {
        match self {
            Expression::Comparison {
                field,
                operator,
                values,
            } => {
                let field_name = if use_qualified {
                    field.qualified_name()
                } else {
                    field.quoted_name()
                };
                let param_count = values.len();
                let sql = match operator {
                    Operator::IsNull | Operator::IsNotNull => {
                        format!("{} {}", field_name, operator.to_sql())
                    }
                    Operator::In => {
                        let params: Vec<String> = (0..param_count)
                            .map(|i| format!("${}", start_param + i))
                            .collect();
                        format!("{} IN ({})", field_name, params.join(", "))
                    }
                    Operator::Between => {
                        format!(
                            "{} BETWEEN ${} AND ${}",
                            field_name,
                            start_param,
                            start_param + 1
                        )
                    }
                    _ => {
                        format!("{} {} ${}", field_name, operator.to_sql(), start_param)
                    }
                };
                (sql, values, start_param + param_count)
            }
            Expression::And(left, right) => {
                let (left_sql, mut left_values, next_param) =
                    left.build_internal_with_qualifier(start_param, use_qualified);
                let (right_sql, right_values, next_param) =
                    right.build_internal_with_qualifier(next_param, use_qualified);
                left_values.extend(right_values);
                (
                    format!("({} AND {})", left_sql, right_sql),
                    left_values,
                    next_param,
                )
            }
            Expression::Or(left, right) => {
                let (left_sql, mut left_values, next_param) =
                    left.build_internal_with_qualifier(start_param, use_qualified);
                let (right_sql, right_values, next_param) =
                    right.build_internal_with_qualifier(next_param, use_qualified);
                left_values.extend(right_values);
                (
                    format!("({} OR {})", left_sql, right_sql),
                    left_values,
                    next_param,
                )
            }
        }
    }

    /// 生成带表名前缀的 SQL（用于 JOIN 场景）
    /// 
    /// 返回包含 SQL 字符串和参数值的 SqlResult
    pub fn build_qualified(self) -> SqlResult {
        let (sql, values, _) = self.build_internal_with_qualifier(1, true);
        SqlResult { sql, values }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id_field() -> FieldInfo {
        FieldInfo::new("id", "users", true)
    }

    fn name_field() -> FieldInfo {
        FieldInfo::new("name", "users", false)
    }

    fn email_field() -> FieldInfo {
        FieldInfo::new("email", "users", false)
    }

    #[test]
    fn test_field_info() {
        let field = id_field();
        assert_eq!(field.quoted_name(), "\"id\"");
        assert_eq!(field.qualified_name(), "users.\"id\"");
    }

    #[test]
    fn test_operator_to_sql() {
        assert_eq!(Operator::Eq.to_sql(), "=");
        assert_eq!(Operator::Ne.to_sql(), "!=");
        assert_eq!(Operator::Like.to_sql(), "LIKE");
        assert_eq!(Operator::IsNull.to_sql(), "IS NULL");
    }

    #[test]
    fn test_simple_comparison() {
        let expr = Expression::comparison(id_field(), Operator::Eq, Value::I32(42));
        let result = expr.build();
        assert_eq!(result.sql, "\"id\" = $1");
        assert_eq!(result.values.len(), 1);
        match &result.values[0] {
            Value::I32(v) => assert_eq!(*v, 42),
            _ => panic!("Expected I32"),
        }
    }

    #[test]
    fn test_like_with_value() {
        let expr = Expression::comparison(
            name_field(),
            Operator::Like,
            Value::String("John%".to_string()),
        );
        let result = expr.build();
        assert_eq!(result.sql, "\"name\" LIKE $1");
        match &result.values[0] {
            Value::String(v) => assert_eq!(v, "John%"),
            _ => panic!("Expected String"),
        }
    }

    #[test]
    fn test_is_null() {
        let expr = Expression::comparison_no_value(email_field(), Operator::IsNull);
        let result = expr.build();
        assert_eq!(result.sql, "\"email\" IS NULL");
        assert!(result.values.is_empty());
    }

    #[test]
    fn test_between() {
        let age_field = FieldInfo::new("age", "users", false);
        let expr = Expression::comparison_multi(
            age_field,
            Operator::Between,
            vec![Value::I32(18), Value::I32(65)],
        );
        let result = expr.build();
        assert_eq!(result.sql, "\"age\" BETWEEN $1 AND $2");
        assert_eq!(result.values.len(), 2);
    }

    #[test]
    fn test_in() {
        let status_field = FieldInfo::new("status", "users", false);
        let expr = Expression::comparison_multi(
            status_field,
            Operator::In,
            vec![Value::I32(1), Value::I32(2), Value::I32(3)],
        );
        let result = expr.build();
        assert_eq!(result.sql, "\"status\" IN ($1, $2, $3)");
        assert_eq!(result.values.len(), 3);
    }

    #[test]
    fn test_and_expression() {
        let left = Expression::comparison(id_field(), Operator::Eq, Value::I32(1));
        let right = Expression::comparison(
            name_field(),
            Operator::Like,
            Value::String("John%".to_string()),
        );
        let expr = left.and(right);
        let result = expr.build();
        assert_eq!(result.sql, "(\"id\" = $1 AND \"name\" LIKE $2)");
        assert_eq!(result.values.len(), 2);
    }

    #[test]
    fn test_complex_expression() {
        let id_eq = Expression::comparison(id_field(), Operator::Eq, Value::I32(1));
        let name_like = Expression::comparison(
            name_field(),
            Operator::Like,
            Value::String("John%".to_string()),
        );
        let email_null = Expression::comparison_no_value(email_field(), Operator::IsNull);

        let expr = id_eq.and(name_like).or(email_null);
        let result = expr.build();
        assert_eq!(
            result.sql,
            "((\"id\" = $1 AND \"name\" LIKE $2) OR \"email\" IS NULL)"
        );
        assert_eq!(result.values.len(), 2);
    }

    #[test]
    fn test_build_qualified() {
        let expr = Expression::comparison(id_field(), Operator::Eq, Value::I32(1));
        let result = expr.build_qualified();
        assert_eq!(result.sql, "users.\"id\" = $1");
    }

    #[test]
    fn test_get_fields() {
        let id_eq = Expression::comparison(id_field(), Operator::Eq, Value::I32(1));
        let name_like = Expression::comparison(
            name_field(),
            Operator::Like,
            Value::String("John%".to_string()),
        );
        let expr = id_eq.and(name_like);
        let fields = expr.fields();
        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0].name, "id");
        assert_eq!(fields[1].name, "name");
    }
}
