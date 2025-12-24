# 迁移到 tokio-postgres 计划

## 📋 概述

将 `conservator` 从 `sqlx` 迁移到 `tokio-postgres`，以降低依赖复杂度、减少编译时间，并获得更直接的 PostgreSQL 协议控制。

## 🎯 迁移目标

1. **保持用户 API 不变** - 用户代码无需修改
2. **保持功能完整性** - 所有现有功能正常工作
3. **降低依赖复杂度** - 移除大型 sqlx 依赖
4. **提升编译速度** - 减少编译时间

## 📊 当前依赖分析

### sqlx 使用情况

| 组件 | 使用位置 | 迁移难度 | 说明 |
|------|---------|---------|------|
| `Executor` trait | 所有 builder 的 async 方法 | 中 | 需要创建自定义 Executor trait |
| `FromRow` trait | `Selectable`、宏生成代码 | 低 | `tokio-postgres::Row` API 相似，只需创建 trait 包装 |
| `query/query_as/query_scalar` | 所有查询执行 | 中 | 需要改为 `prepare` + `execute/query_one/query` |
| `PgRow` | 行类型 | 低 | 改为 `tokio_postgres::Row` |
| `Error` | 错误处理 | 低 | 需要错误类型转换 |
| `Pool` | 连接池 | 中 | 使用 `deadpool-postgres` |
| `migrate` | 数据库迁移 | 低 | 可选，可移除或使用替代方案 |

## 🔧 迁移步骤

### 阶段 1: 核心抽象层（3-5 天）

#### 1.1 创建自定义 Executor trait

**文件**: `conservator/src/executor.rs` (新建)

```rust
use tokio_postgres::{Client, Transaction};
use tokio_postgres::Error as PgError;

/// 抽象执行器，统一 Client 和 Transaction
pub trait Executor {
    async fn execute(&self, query: &str, params: &[&(dyn tokio_postgres::types::ToSql + Sync)]) 
        -> Result<u64, PgError>;
    
    async fn query_one(&self, query: &str, params: &[&(dyn tokio_postgres::types::ToSql + Sync)]) 
        -> Result<tokio_postgres::Row, PgError>;
    
    async fn query(&self, query: &str, params: &[&(dyn tokio_postgres::types::ToSql + Sync)]) 
        -> Result<Vec<tokio_postgres::Row>, PgError>;
}

// 为 Client 实现
impl Executor for Client {
    // ... 实现
}

// 为 Transaction 实现
impl<'a> Executor for Transaction<'a> {
    // ... 实现
}
```

**任务**:
- [ ] 创建 `executor.rs` 文件
- [ ] 定义 `Executor` trait
- [ ] 为 `Client` 实现 trait
- [ ] 为 `Transaction<'a>` 实现 trait
- [ ] 编写单元测试

#### 1.2 创建自定义 FromRow trait

**文件**: `conservator/src/from_row.rs` (新建)

```rust
use tokio_postgres::{Row, Error as PgError};

/// 从 Row 转换为类型的 trait
pub trait FromRow: Sized {
    fn from_row(row: &Row) -> Result<Self, PgError>;
}
```

**任务**:
- [ ] 创建 `from_row.rs` 文件
- [ ] 定义 `FromRow` trait
- [ ] 更新 `Selectable` trait 约束
- [ ] 更新 `Domain` trait 约束

#### 1.3 错误类型转换

**文件**: `conservator/src/error.rs` (新建)

```rust
use tokio_postgres::Error as PgError;

/// 统一的错误类型（目前直接使用 PgError，未来可扩展）
pub type ConservatorError = PgError;

// 如果需要，可以创建自定义错误类型包装
```

**任务**:
- [ ] 创建 `error.rs` 文件
- [ ] 定义错误类型别名或包装
- [ ] 更新所有返回类型

### 阶段 2: 更新核心类型（2-3 天）

#### 2.1 更新 Selectable trait

**文件**: `conservator/src/lib.rs`

```rust
// 修改前
pub trait Selectable:
    Sized + Send + Unpin + for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow>
{
    const COLUMN_NAMES: &'static [&'static str];
}

// 修改后
pub trait Selectable: Sized + Send + Unpin + FromRow {
    const COLUMN_NAMES: &'static [&'static str];
}
```

**任务**:
- [ ] 更新 `Selectable` trait 定义
- [ ] 移除 `sqlx::FromRow` 依赖
- [ ] 添加 `FromRow` 依赖

#### 2.2 更新 Domain trait

**文件**: `conservator/src/lib.rs`

```rust
// 修改 Executor 约束
async fn fetch_one_by_pk<'e, 'c: 'e, E: 'e + Executor>(
    pk: &Self::PrimaryKey,
    executor: E,
) -> Result<Self, ConservatorError>;
```

**任务**:
- [ ] 更新所有 `Domain` 方法签名
- [ ] 替换 `sqlx::Executor` 为自定义 `Executor`
- [ ] 替换 `sqlx::Error` 为 `ConservatorError`

#### 2.3 更新 Value 和参数绑定

**文件**: `conservator/src/value.rs`

```rust
// 修改前
impl Value {
    pub fn bind_to_query<'q>(self, query: sqlx::query::QueryAs<...>) -> ... {
        // sqlx 绑定
    }
}

// 修改后
impl Value {
    pub fn to_sql_param(&self) -> Box<dyn tokio_postgres::types::ToSql + Sync + Send> {
        match self {
            Value::I32(v) => Box::new(*v),
            Value::String(v) => Box::new(v.clone()),
            // ... 其他类型
        }
    }
}
```

**任务**:
- [ ] 移除 `bind_to_query` 方法
- [ ] 实现 `ToSql` trait 转换
- [ ] 处理所有 Value 变体的转换

### 阶段 3: 更新 Builder（5-7 天）

#### 3.1 更新 SelectBuilder

**文件**: `conservator/src/builder/select.rs`

```rust
// 修改前
pub async fn one<'e, 'c: 'e, E: 'e + sqlx::Executor<'c, Database = sqlx::Postgres>>(
    self,
    executor: E,
) -> Result<Returning, sqlx::Error> {
    let sql_result = self.build();
    let mut query = sqlx::query_as::<_, Returning>(&sql_result.sql);
    for value in sql_result.values {
        query = value.bind_to(query);
    }
    query.fetch_one(executor).await
}

// 修改后
pub async fn one<E: Executor>(
    self,
    executor: &E,
) -> Result<Returning, ConservatorError> {
    let sql_result = self.build();
    
    // 准备参数
    let params: Vec<Box<dyn tokio_postgres::types::ToSql + Sync + Send>> = 
        sql_result.values.iter()
            .map(|v| v.to_sql_param())
            .collect();
    
    // 转换为引用数组
    let param_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = 
        params.iter().map(|p| p.as_ref()).collect();
    
    // 执行查询
    let row = executor.query_one(&sql_result.sql, &param_refs).await?;
    Returning::from_row(&row)
}
```

**任务**:
- [ ] 更新 `one()` 方法
- [ ] 更新 `all()` 方法
- [ ] 更新 `optional()` 方法
- [ ] 处理参数绑定转换
- [ ] 更新测试

#### 3.2 更新 InsertBuilder

**文件**: `conservator/src/builder/insert.rs`

```rust
// 修改 returning_pk
pub async fn returning_pk<E: Executor>(
    self,
    executor: &E,
) -> Result<T::PrimaryKey, ConservatorError> {
    // 使用 query_one 获取单行，然后提取 PK
}

// 修改 returning_entity
pub async fn returning_entity<E: Executor>(
    self,
    executor: &E,
) -> Result<T, ConservatorError> {
    // 使用 query_one 获取单行，然后 FromRow
}
```

**任务**:
- [ ] 更新 `returning_pk()` 方法
- [ ] 更新 `returning_entity()` 方法
- [ ] 更新 `execute()` 方法
- [ ] 处理批量插入的参数绑定

#### 3.3 更新 UpdateBuilder

**文件**: `conservator/src/builder/update.rs`

```rust
pub async fn execute<E: Executor>(
    self,
    executor: &E,
) -> Result<u64, ConservatorError> {
    let sql_result = self.build();
    let params: Vec<Box<dyn tokio_postgres::types::ToSql + Sync + Send>> = 
        sql_result.values.iter().map(|v| v.to_sql_param()).collect();
    let param_refs: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> = 
        params.iter().map(|p| p.as_ref()).collect();
    
    executor.execute(&sql_result.sql, &param_refs).await
}
```

**任务**:
- [ ] 更新 `execute()` 方法
- [ ] 更新测试

#### 3.4 更新 DeleteBuilder

**文件**: `conservator/src/builder/delete.rs`

类似 UpdateBuilder 的修改。

**任务**:
- [ ] 更新 `execute()` 方法
- [ ] 更新测试

### 阶段 4: 更新宏生成代码（3-4 天）

#### 4.1 更新 Selectable 宏

**文件**: `conservator_macro/src/selectable.rs`

```rust
// 修改前
impl<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> for #ident {
    fn from_row(row: &'r sqlx::postgres::PgRow) -> Result<Self, sqlx::Error> {
        use sqlx::Row;
        Ok(Self {
            #(#from_row_fields),*
        })
    }
}

// 修改后
impl ::conservator::FromRow for #ident {
    fn from_row(row: &tokio_postgres::Row) -> Result<Self, tokio_postgres::Error> {
        Ok(Self {
            id: row.try_get("id")?,
            name: row.try_get("name")?,
            // ...
        })
    }
}
```

**任务**:
- [ ] 更新 `derive_selectable_fn` 宏
- [ ] 修改生成的 `FromRow` 实现
- [ ] 使用 `tokio_postgres::Row::try_get()`
- [ ] 更新错误类型

#### 4.2 更新 Domain 宏

**文件**: `conservator_macro/src/domain.rs`

```rust
// 修改 update 方法生成
async fn update<'e, 'c: 'e, E: 'e + ::conservator::Executor>(
    &self,
    executor: &E,
) -> Result<(), ::conservator::ConservatorError> {
    // 使用新的 Executor trait
}
```

**任务**:
- [ ] 更新 `derive_domain_fn` 宏
- [ ] 修改生成的 `update()` 方法
- [ ] 修改生成的 `fetch_one_by_pk()` 等方法
- [ ] 更新所有 Executor 约束

#### 4.3 更新 Creatable 宏

**文件**: `conservator_macro/src/creatable.rs`

需要移除 `build_for_query_as` 和 `build_for_query` 方法，改为直接生成参数数组。

**任务**:
- [ ] 更新 `derive_creatable_fn` 宏
- [ ] 移除 sqlx 特定的方法
- [ ] 添加 `to_sql_params()` 方法生成

### 阶段 5: 连接池和迁移工具（2-3 天）

#### 5.1 更新连接池

**文件**: `conservator/src/lib.rs`

```rust
// 修改前
pub use sqlx::Pool;
pub use sqlx::postgres::PgPoolOptions;

// 修改后
pub use deadpool_postgres::{Pool, PoolConfig, Runtime};
pub use deadpool_postgres::Config as PoolConfigBuilder;

// 或者创建包装类型以保持 API 兼容
pub type PgPool = deadpool_postgres::Pool;
```

**任务**:
- [ ] 添加 `deadpool-postgres` 依赖
- [ ] 更新 `lib.rs` 导出
- [ ] 创建连接池辅助函数（如果需要）
- [ ] 更新文档

#### 5.2 处理数据库迁移

**选项 1**: 移除迁移支持（推荐）
- 用户可以使用 `refinery` 或其他迁移工具

**选项 2**: 创建简单的迁移包装
- 基于 `tokio-postgres` 实现基本迁移功能

**任务**:
- [ ] 决定迁移策略
- [ ] 如果移除，更新文档说明
- [ ] 如果保留，实现基本迁移功能

### 阶段 6: 依赖更新（1 天）

#### 6.1 更新 Cargo.toml

**文件**: `conservator/Cargo.toml`

```toml
[dependencies]
# 移除
# sqlx = { ... }

# 添加
tokio-postgres = { version = "0.7", features = ["with-chrono-0_4", "with-uuid-1", "with-serde_json-1"] }
deadpool-postgres = "0.10"
chrono = "0.4"  # 保留，用于 Value 类型
bigdecimal = "0.4"  # 可能需要 tokio-postgres 的 bigdecimal 支持
uuid = { version = "1", features = ["v4"] }
serde_json = "1"
```

**任务**:
- [ ] 移除 `sqlx` 依赖
- [ ] 添加 `tokio-postgres` 依赖
- [ ] 添加 `deadpool-postgres` 依赖
- [ ] 检查所有 feature flags
- [ ] 更新 `conservator_macro/Cargo.toml`（如果有依赖）

### 阶段 7: 测试和验证（3-5 天）

#### 7.1 更新单元测试

**任务**:
- [ ] 更新所有 mock 测试
- [ ] 更新 Executor trait 的测试
- [ ] 验证所有 builder 的测试通过

#### 7.2 更新集成测试

**文件**: `conservator/tests/integration.rs`

```rust
// 修改前
use sqlx::PgPool;

// 修改后
use deadpool_postgres::Pool as PgPool;
```

**任务**:
- [ ] 更新测试中的连接池创建
- [ ] 更新所有测试用例
- [ ] 验证所有集成测试通过
- [ ] 性能对比测试（可选）

#### 7.3 兼容性测试

**任务**:
- [ ] 测试所有现有功能
- [ ] 验证用户代码无需修改（API 兼容）
- [ ] 性能基准测试
- [ ] 内存使用测试

### 阶段 8: 文档更新（1-2 天）

#### 8.1 更新 README

**任务**:
- [ ] 更新依赖说明
- [ ] 更新连接池示例
- [ ] 更新迁移说明（如果移除）
- [ ] 添加迁移指南（从 sqlx 迁移）

#### 8.2 更新 CHANGELOG

**任务**:
- [ ] 记录重大变更
- [ ] 说明迁移步骤
- [ ] 列出破坏性变更（如果有）

## 📦 依赖变更

### 移除
- `sqlx` (大型依赖，包含编译时 SQL 检查等)

### 添加
- `tokio-postgres` (轻量级 PostgreSQL 客户端)
- `deadpool-postgres` (连接池)

### 保留
- `chrono` (日期时间类型)
- `bigdecimal` (大数类型)
- `uuid` (UUID 类型)
- `serde_json` (JSON 类型)

## ⚠️ 潜在问题和解决方案

### 问题 1: 参数绑定生命周期

**问题**: `tokio-postgres` 的参数绑定需要引用，而我们的 `Value` 是 owned 类型。

**解决方案**: 
- 使用 `Box<dyn ToSql>` 存储参数
- 在执行前转换为引用数组
- 或者重构为使用引用

### 问题 2: 事务支持

**问题**: `sqlx::Executor` 同时支持 `Pool` 和 `Transaction`，需要确保我们的 `Executor` trait 也能做到。

**解决方案**: 
- 为 `Transaction<'a>` 实现 `Executor`
- 确保生命周期正确

### 问题 3: 类型转换

**问题**: `tokio-postgres` 的类型系统与 `sqlx` 不同，需要确保所有类型都能正确转换。

**解决方案**:
- 为所有 `Value` 变体实现 `ToSql`
- 测试所有数据类型
- 必要时添加类型转换层

### 问题 4: 编译时 SQL 检查

**问题**: `sqlx` 提供编译时 SQL 检查，`tokio-postgres` 没有。

**解决方案**:
- 这是迁移的权衡，接受运行时检查
- 可以通过测试覆盖 SQL 正确性
- 未来可以考虑添加 SQL 验证工具

## 📈 预期收益

1. **编译时间**: 减少 30-50%（移除大型 sqlx 依赖）
2. **二进制大小**: 减少 10-20%（移除未使用的 sqlx 功能）
3. **依赖数量**: 减少 1 个主要依赖
4. **控制力**: 更直接的 PostgreSQL 协议控制

## 📅 时间估算

| 阶段 | 时间 | 累计 |
|------|------|------|
| 阶段 1: 核心抽象层 | 3-5 天 | 3-5 天 |
| 阶段 2: 更新核心类型 | 2-3 天 | 5-8 天 |
| 阶段 3: 更新 Builder | 5-7 天 | 10-15 天 |
| 阶段 4: 更新宏 | 3-4 天 | 13-19 天 |
| 阶段 5: 连接池和迁移 | 2-3 天 | 15-22 天 |
| 阶段 6: 依赖更新 | 1 天 | 16-23 天 |
| 阶段 7: 测试和验证 | 3-5 天 | 19-28 天 |
| 阶段 8: 文档更新 | 1-2 天 | 20-30 天 |

**总计**: 约 4-6 周

## ✅ 验收标准

1. ✅ 所有现有测试通过
2. ✅ 用户 API 保持不变（向后兼容）
3. ✅ 所有功能正常工作
4. ✅ 编译时间减少
5. ✅ 文档完整更新
6. ✅ 性能不低于 sqlx 版本（或可接受的性能差异）

## 🚀 开始迁移

建议按阶段逐步迁移，每个阶段完成后进行测试，确保稳定性。

**第一步**: 创建 feature flag `tokio-postgres`，允许同时支持两个后端（可选，用于平滑迁移）

**或者**: 直接替换，因为 API 保持兼容，用户代码无需修改。

