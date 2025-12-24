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
