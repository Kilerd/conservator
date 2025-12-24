# Contributing to Conservator

Thank you for your interest in contributing to Conservator! This document provides guidelines and instructions for contributing to the project.

## Development Setup

### Prerequisites

- Rust 1.75 or later
- PostgreSQL 13+ (for integration tests)
- Docker (optional, for testcontainers-based tests)

### Getting Started

1. Fork and clone the repository:
```bash
git clone https://github.com/kilerd/conservator.git
cd conservator
```

2. Build the project:
```bash
cargo build
```

3. Run tests:
```bash
# Unit tests
cargo test --lib

# Integration tests (requires PostgreSQL via Docker)
cargo test --test integration

# Compile-time tests
cargo test --test trybuild

# All tests
cargo test
```

## Project Structure

```
conservator/
├── conservator/              # Main library crate
│   ├── src/
│   │   ├── builder/         # Query builders (select, insert, update, delete)
│   │   ├── executor.rs      # Unified executor abstraction
│   │   ├── error.rs         # Error types
│   │   ├── expression.rs    # SQL expression system
│   │   ├── field.rs         # Field metadata and type safety
│   │   ├── migrate.rs       # Migration system
│   │   └── conn.rs          # Connection pool management
│   └── tests/
│       ├── integration/     # Integration tests
│       ├── pass/            # Compile-time pass tests
│       └── fail/            # Compile-time fail tests
└── conservator_macro/        # Procedural macros
    └── src/
        ├── domain.rs        # #[derive(Domain)] macro
        ├── selectable.rs    # #[derive(Selectable)] macro
        └── creatable.rs     # #[derive(Creatable)] macro
```

## Code Style

### Formatting

We use `rustfmt` for code formatting. Before submitting a PR, run:

```bash
cargo fmt --all
```

### Linting

We use `clippy` with strict settings. Ensure your code passes:

```bash
cargo clippy --all-targets -- -D warnings
```

### Naming Conventions

- Use `snake_case` for functions, variables, and module names
- Use `PascalCase` for types, traits, and enum variants
- Use `SCREAMING_SNAKE_CASE` for constants

### Code Comments

- Add doc comments (`///`) for all public APIs
- Use inline comments (`//`) to explain complex logic
- Prefer code clarity over comments when possible

## Testing

### Writing Tests

- **Unit tests**: Place in the same file as the code being tested
- **Integration tests**: Place in `tests/integration/`
- **Compile-time tests**: Place in `tests/pass/` or `tests/fail/`

### Test Guidelines

- Each test should be independent and idempotent
- Use descriptive test names: `test_<what>_<condition>_<expected_result>`
- Integration tests use testcontainers for isolated PostgreSQL instances
- Clean up resources properly (connections, test data)

Example:
```rust
#[test]
fn test_select_with_filter_returns_matching_rows() {
    // Arrange
    let expr = User::COLUMNS.id.eq(1);
    let builder = User::select().filter(expr);

    // Act
    let sql = builder.build();

    // Assert
    assert!(sql.sql.contains("WHERE"));
}
```

## Submitting Changes

### Pull Request Process

1. **Create a branch**: Use a descriptive branch name
   - Feature: `feat/add-custom-types`
   - Bug fix: `fix/connection-leak`
   - Refactor: `refactor/cleanup-builder`

2. **Make your changes**:
   - Write tests for new functionality
   - Update documentation if needed
   - Ensure all tests pass
   - Run `cargo fmt` and `cargo clippy`

3. **Commit your changes**:
   - Use conventional commit messages:
     - `feat:` for new features
     - `fix:` for bug fixes
     - `refactor:` for code refactoring
     - `docs:` for documentation changes
     - `test:` for test additions/changes
     - `chore:` for maintenance tasks
   - Example: `feat: add support for JSONB operators`

4. **Push to your fork**:
```bash
git push origin feat/your-feature
```

5. **Create a Pull Request**:
   - Provide a clear description of the changes
   - Reference any related issues
   - Include test results if applicable

### PR Requirements

Before your PR can be merged:

- [ ] All tests pass
- [ ] Code is formatted with `rustfmt`
- [ ] No warnings from `clippy`
- [ ] New functionality has tests
- [ ] Documentation is updated (if applicable)
- [ ] CHANGELOG.md is updated (for significant changes)

## Reporting Issues

### Bug Reports

When reporting bugs, please include:

1. **Description**: Clear and concise description of the bug
2. **Reproduction steps**: Minimal code example to reproduce
3. **Expected behavior**: What you expected to happen
4. **Actual behavior**: What actually happened
5. **Environment**:
   - Rust version: `rustc --version`
   - conservator version
   - PostgreSQL version
   - Operating system

Example:
```markdown
## Bug Description
Connection pool hangs after 10 concurrent queries

## Steps to Reproduce
\`\`\`rust
let pool = PooledConnection::from_url("...")?;
for _ in 0..20 {
    tokio::spawn(async move {
        let user = User::fetch_one_by_pk(&1, &pool).await?;
    });
}
\`\`\`

## Expected
All queries complete successfully

## Actual
Hangs after 10 queries

## Environment
- Rust: 1.75.0
- conservator: 0.2.0
- PostgreSQL: 15.3
- OS: macOS 14.0
```

### Feature Requests

For feature requests, please include:

1. **Use case**: Why do you need this feature?
2. **Proposed solution**: How would you like it to work?
3. **Alternatives**: Any workarounds or alternative approaches?

## Development Guidelines

### Performance Considerations

- Avoid unnecessary allocations
- Use zero-cost abstractions where possible
- Profile changes that may impact performance
- Consider both compile-time and runtime performance

### Type Safety

- Leverage Rust's type system for correctness
- Use type-state pattern to prevent invalid states
- Prefer compile-time errors over runtime errors

### API Design

- Keep APIs simple and intuitive
- Maintain consistency with existing patterns
- Consider both ease of use and flexibility
- Document edge cases and limitations

## Questions?

If you have questions about contributing:

- Open an issue with the `question` label
- Check existing issues and discussions
- Review the documentation at [docs]

Thank you for contributing to Conservator!
