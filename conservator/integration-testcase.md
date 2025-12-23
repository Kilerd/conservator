# 集成测试用例

本文档列出 `conservator` 的所有集成测试用例及其简介。

## CRUD 基础测试

| 测试名称 | 简介 |
|----------|------|
| `test_insert_returning_pk` | 插入记录并返回主键 |
| `test_insert_returning_entity` | 插入记录并返回完整实体 |
| `test_fetch_by_pk` | 通过主键获取单条记录 |
| `test_entity_update` | Active Record 风格的实体更新 |

## 表达式操作符测试

| 测试名称 | 简介 |
|----------|------|
| `test_eq_operator` | 等于操作符 `=` |
| `test_ne_operator` | 不等于操作符 `!=` |
| `test_gt_operator` | 大于操作符 `>` |
| `test_lt_operator` | 小于操作符 `<` |
| `test_gte_operator` | 大于等于操作符 `>=` |
| `test_lte_operator` | 小于等于操作符 `<=` |
| `test_between_operator` | 范围操作符 `BETWEEN` |
| `test_in_list_operator` | 集合操作符 `IN` (字符串) |
| `test_in_list_with_integers` | 集合操作符 `IN` (整数) |
| `test_is_null_operator` | 空值判断 `IS NULL` |
| `test_is_not_null_operator` | 非空判断 `IS NOT NULL` |
| `test_like_operator` | 模糊匹配 `LIKE` (后缀匹配) |
| `test_like_operator_prefix` | 模糊匹配 `LIKE` (前缀匹配) |

## 复杂表达式组合测试

| 测试名称 | 简介 |
|----------|------|
| `test_and_combination` | `.and()` 方法组合表达式 |
| `test_or_combination` | `.or()` 方法组合表达式 |
| `test_bitand_operator` | `&` 运算符组合表达式 (等价于 AND) |
| `test_bitor_operator` | `\|` 运算符组合表达式 (等价于 OR) |
| `test_nested_expressions` | 嵌套括号组合表达式 |
| `test_multiple_filter_calls` | 多次调用 `.filter()` 链式组合 |

## 排序测试

| 测试名称 | 简介 |
|----------|------|
| `test_order_by_asc` | 升序排序 |
| `test_order_by_desc` | 降序排序 |
| `test_order_by_multiple_fields` | 双字段排序 (is_active DESC, name ASC) |
| `test_order_by_three_fields` | 三字段排序 (name ASC, age DESC, score ASC) |
| `test_order_by_mixed_asc_desc` | 混合升降序 (score DESC, age ASC) |
| `test_order_by_with_same_values` | 相同值时的次级排序 |
| `test_order_by_with_filter_and_limit` | 过滤 + 多重排序 + 分页组合 |
| `test_order_by_with_limit` | 排序结合分页 (Top N) |

## 分页测试

| 测试名称 | 简介 |
|----------|------|
| `test_limit_offset_pagination` | LIMIT/OFFSET 分页查询 |

## 边界条件测试

| 测试名称 | 简介 |
|----------|------|
| `test_empty_result` | 查询结果为空 |
| `test_optional_not_found` | `find_by_pk` 未找到返回 None |
| `test_one_not_found_error` | `.one()` 未找到返回错误 |
| `test_optional_found` | `.optional()` 找到返回 Some |
| `test_optional_not_found_returns_none` | `.optional()` 未找到返回 None |
| `test_special_characters_in_string` | 特殊字符处理 (单引号) |
| `test_unicode_characters` | Unicode 字符支持 (中文) |
| `test_empty_string` | 空字符串处理 |

## 多数据类型测试

| 测试名称 | 简介 |
|----------|------|
| `test_uuid_type` | UUID 类型支持 |
| `test_bigdecimal_type` | BigDecimal 精确数值类型 |
| `test_json_type` | JSON/JSONB 类型支持 |
| `test_datetime_type` | DateTime 时间戳类型 |

## Projection 测试

| 测试名称 | 简介 |
|----------|------|
| `test_returning_projection` | 使用 `.returning()` 返回投影类型 |
| `test_returning_with_filter` | 投影结合过滤和排序 |

## Delete 测试

| 测试名称 | 简介 |
|----------|------|
| `test_delete_single` | 删除单条记录 |
| `test_delete_multiple` | 批量删除 (按条件) |
| `test_delete_with_complex_filter` | 复杂过滤条件删除 |

## Update 测试

| 测试名称 | 简介 |
|----------|------|
| `test_update_single_field` | 更新单个字段 |
| `test_update_multiple_fields` | 更新多个字段 |
| `test_update_multiple_rows` | 批量更新多行 |

## 批量操作测试

| 测试名称 | 简介 |
|----------|------|
| `test_insert_many_returning_pks` | 批量插入返回主键列表 |
| `test_insert_many_returning_entities` | 批量插入返回实体列表 |

## 事务测试

| 测试名称 | 简介 |
|----------|------|
| `test_transaction_commit` | 事务提交 |
| `test_transaction_rollback` | 事务回滚 |
| `test_transaction_multiple_operations` | 单事务内多操作 (增删改) |

## 日期时间类型操作符测试

| 测试名称 | 简介 |
|----------|------|
| `test_datetime_gt_operator` | DateTime 大于比较 (7天内) |
| `test_datetime_lt_operator` | DateTime 小于比较 (14天前) |
| `test_datetime_gte_operator` | DateTime 大于等于比较 |
| `test_datetime_lte_operator` | DateTime 小于等于比较 |
| `test_datetime_between_operator` | DateTime 范围查询 (7-14天前) |
| `test_datetime_order_by` | DateTime 排序 (ASC/DESC) |
| `test_datetime_combined_with_other_filters` | DateTime 结合其他过滤条件 |
| `test_datetime_in_update` | 使用 DateTime 条件更新 |
| `test_datetime_in_delete` | 使用 DateTime 条件删除 |

## 其他测试

| 测试名称 | 简介 |
|----------|------|
| `test_float_comparison` | 浮点数范围比较 |
| `test_composite_condition_as_unique_identifier` | 复合条件模拟唯一约束查询 |
| `test_bulk_insert_and_query` | 大数据量批量插入与复杂查询 (100条) |

---

**总计: 69 个测试用例**

