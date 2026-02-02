// Integration test entry point
// Explicitly specify paths for submodules in integration/ directory

#[path = "integration/crud.rs"]
mod crud;

#[path = "integration/migrate.rs"]
mod migrate;

#[path = "integration/sql_macro.rs"]
mod sql_macro;

#[path = "integration/executor_refs.rs"]
mod executor_refs;

#[path = "integration/query_functions.rs"]
mod query_functions;

#[path = "integration/vector_types_simple.rs"]
mod vector_types_simple;
