# Integration Test Cases

This document lists all integration test cases for `conservator` and their descriptions.

## CRUD Basic Tests

| Test Name | Description |
|-----------|-------------|
| `test_insert_returning_pk` | Insert record and return primary key |
| `test_insert_returning_entity` | Insert record and return complete entity |
| `test_fetch_by_pk` | Fetch single record by primary key |
| `test_entity_update` | Active Record style entity update |

## Expression Operator Tests

| Test Name | Description |
|-----------|-------------|
| `test_eq_operator` | Equality operator `=` |
| `test_ne_operator` | Inequality operator `!=` |
| `test_gt_operator` | Greater than operator `>` |
| `test_lt_operator` | Less than operator `<` |
| `test_gte_operator` | Greater than or equal operator `>=` |
| `test_lte_operator` | Less than or equal operator `<=` |
| `test_between_operator` | Range operator `BETWEEN` |
| `test_in_list_operator` | Set operator `IN` (strings) |
| `test_in_list_with_integers` | Set operator `IN` (integers) |
| `test_is_null_operator` | NULL check `IS NULL` |
| `test_is_not_null_operator` | NOT NULL check `IS NOT NULL` |
| `test_like_operator` | Pattern matching `LIKE` (suffix match) |
| `test_like_operator_prefix` | Pattern matching `LIKE` (prefix match) |

## Complex Expression Combination Tests

| Test Name | Description |
|-----------|-------------|
| `test_and_combination` | Combine expressions with `.and()` method |
| `test_or_combination` | Combine expressions with `.or()` method |
| `test_bitand_operator` | Combine expressions with `&` operator (equivalent to AND) |
| `test_bitor_operator` | Combine expressions with `\|` operator (equivalent to OR) |
| `test_nested_expressions` | Nested parentheses combination |
| `test_multiple_filter_calls` | Chain multiple `.filter()` calls |

## Ordering Tests

| Test Name | Description |
|-----------|-------------|
| `test_order_by_asc` | Ascending order |
| `test_order_by_desc` | Descending order |
| `test_order_by_multiple_fields` | Two-field sorting (is_active DESC, name ASC) |
| `test_order_by_three_fields` | Three-field sorting (name ASC, age DESC, score ASC) |
| `test_order_by_mixed_asc_desc` | Mixed ascending/descending (score DESC, age ASC) |
| `test_order_by_with_same_values` | Secondary sorting when values are equal |
| `test_order_by_with_filter_and_limit` | Filter + multiple sorting + pagination combination |
| `test_order_by_with_limit` | Sorting with pagination (Top N) |

## Pagination Tests

| Test Name | Description |
|-----------|-------------|
| `test_limit_offset_pagination` | LIMIT/OFFSET pagination query |

## Edge Case Tests

| Test Name | Description |
|-----------|-------------|
| `test_empty_result` | Empty query result |
| `test_optional_not_found` | `find_by_pk` returns None when not found |
| `test_one_not_found_error` | `.one()` returns error when not found |
| `test_optional_found` | `.optional()` returns Some when found |
| `test_optional_not_found_returns_none` | `.optional()` returns None when not found |
| `test_special_characters_in_string` | Special character handling (single quote) |
| `test_unicode_characters` | Unicode character support (Chinese) |
| `test_empty_string` | Empty string handling |

## Multi-DataType Tests

| Test Name | Description |
|-----------|-------------|
| `test_uuid_type` | UUID type support |
| `test_bigdecimal_type` | BigDecimal precise numeric type |
| `test_json_type` | JSON/JSONB type support |
| `test_datetime_type` | DateTime timestamp type |

## Projection Tests

| Test Name | Description |
|-----------|-------------|
| `test_returning_projection` | Use `.returning()` to return projection type |
| `test_returning_with_filter` | Projection combined with filter and sorting |

## Delete Tests

| Test Name | Description |
|-----------|-------------|
| `test_delete_single` | Delete single record |
| `test_delete_multiple` | Batch delete (by condition) |
| `test_delete_with_complex_filter` | Delete with complex filter conditions |

## Update Tests

| Test Name | Description |
|-----------|-------------|
| `test_update_single_field` | Update single field |
| `test_update_multiple_fields` | Update multiple fields |
| `test_update_multiple_rows` | Batch update multiple rows |

## Batch Operation Tests

| Test Name | Description |
|-----------|-------------|
| `test_insert_many_returning_pks` | Batch insert returning list of primary keys |
| `test_insert_many_returning_entities` | Batch insert returning list of entities |

## Transaction Tests

| Test Name | Description |
|-----------|-------------|
| `test_transaction_commit` | Transaction commit |
| `test_transaction_rollback` | Transaction rollback |
| `test_transaction_multiple_operations` | Multiple operations in single transaction (insert, update, delete) |

## DateTime Type Operator Tests

| Test Name | Description |
|-----------|-------------|
| `test_datetime_gt_operator` | DateTime greater than comparison (within 7 days) |
| `test_datetime_lt_operator` | DateTime less than comparison (14 days ago) |
| `test_datetime_gte_operator` | DateTime greater than or equal comparison |
| `test_datetime_lte_operator` | DateTime less than or equal comparison |
| `test_datetime_between_operator` | DateTime range query (7-14 days ago) |
| `test_datetime_order_by` | DateTime sorting (ASC/DESC) |
| `test_datetime_combined_with_other_filters` | DateTime combined with other filter conditions |
| `test_datetime_in_update` | Update using DateTime condition |
| `test_datetime_in_delete` | Delete using DateTime condition |

## Other Tests

| Test Name | Description |
|-----------|-------------|
| `test_float_comparison` | Floating point range comparison |
| `test_composite_condition_as_unique_identifier` | Composite condition simulating unique constraint query |
| `test_bulk_insert_and_query` | Large-scale batch insert and complex query (100 records) |

---

**Total: 69 test cases**
