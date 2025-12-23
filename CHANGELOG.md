# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2025-12-24

### Added

#### Core Features
- **Selectable trait**: Lightweight projection type for custom return types with `.returning()`
  - `#[derive(Selectable)]` macro automatically generates `Selectable` and `sqlx::FromRow` implementations
  - Allows returning custom projection types in SELECT queries, not just full Domain entities

- **Simplified order_by API**: More intuitive sorting methods
  - `.order_by(field)` - default ascending order
  - `.order_by(field.asc())` - explicit ascending order
  - `.order_by(field.desc())` - explicit descending order
  - Replaces the previous `.order_by(field, Order::Asc)` syntax

- **Complete expression system**: Type-safe SQL WHERE clause building
  - All common operators: `eq`, `ne`, `gt`, `lt`, `gte`, `lte`
  - Range operations: `between`, `in_list`, `like`
  - NULL checks: `is_null`, `is_not_null` (for `Option<T>` fields)
  - Logical combinations: `.and()`, `.or()`, `&`, `|`
  - Nested expressions support

- **Batch insert operations**: `insert_many()` method
  - Supports inserting multiple records in a single operation
  - Supports `returning_pk()` and `returning_entity()`

- **Active Record style updates**: `entity.update(&db)` method
  - Allows fetching an entity, modifying it, and saving directly
  - Automatically updates all non-primary-key fields

- **Extended data type support**: Enhanced `Value` enum
  - `chrono` time types: `NaiveDate`, `NaiveTime`, `NaiveDateTime`, `DateTime<Utc>`, `DateTime<FixedOffset>`
  - `BigDecimal` for precise numeric types
  - `UUID` type support
  - `JSON/JSONB` types (serde_json::Value)
  - `Option<T>` type support

#### Query Builders
- **SelectBuilder**: Type-safe SELECT query builder
  - `.filter()` - WHERE conditions
  - `.order_by()` - sorting (supports multiple fields)
  - `.limit()` / `.offset()` - pagination
  - `.group_by()` - grouping
  - `.returning()` - custom return types
  - `.one()`, `.optional()`, `.all()` - execution methods

- **InsertBuilder**: Single insert builder
  - `.returning_pk()` - returns primary key
  - `.returning_entity()` - returns complete entity

- **InsertManyBuilder**: Batch insert builder
  - `.returning_pk()` - returns list of primary keys
  - `.returning_entity()` - returns list of entities

- **UpdateBuilder**: Type-safe UPDATE builder
  - Uses typestate pattern to ensure both `.set()` and `.filter()` must be called
  - Prevents accidental updates of all rows

- **DeleteBuilder**: Type-safe DELETE builder
  - Uses typestate pattern to ensure `.filter()` must be called
  - Prevents accidental deletion of all rows

#### Testing
- **Integration tests**: Complete test suite using testcontainers-rs
  - 69 integration test cases
  - Covers all CRUD operations
  - Covers all expression operators
  - Covers complex query scenarios
  - Covers transaction operations
  - Supports parallel execution (each test uses an isolated database)

#### CI/CD
- **GitHub Actions**: Complete CI workflow
  - Code checks (clippy, fmt)
  - Unit tests
  - Documentation tests
  - Integration tests
  - Compile-time tests

### Changed

- **Domain trait**: Now automatically implements `Selectable` and `sqlx::FromRow`
  - No longer requires separate `#[derive(FromRow)]`
  - `Domain` inherits from `Selectable`

- **Expression building**: All field methods now return `Expression` type
  - More unified API
  - Better type safety

- **Code organization**: Moved `Value` enum and `IntoValue` trait to separate `value.rs` module

### Fixed

- Fixed `Domain::update()` method signature to use `&self` instead of `Self`
- Fixed all clippy warnings
- Fixed code formatting issues

### Documentation

- Updated README.md with examples for all new features
- Created `integration-testcase.md` documenting all test cases
- All code examples verified and runnable

### Internal

- Improved macro-generated code quality
- Optimized type constraints and error messages
- Added `Default` implementations to `DeleteBuilder` and `UpdateBuilder`

## [0.1.7] - Previous Version

Previous versions are not documented in this changelog.

---

[0.2.0]: https://github.com/kilerd/conservator/compare/v0.1.7...v0.2.0
