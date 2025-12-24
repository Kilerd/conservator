//! Database Migration System
//!
//! A lightweight migration system for PostgreSQL, built with conservator's own ORM.
//!
//! # Example
//!
//! ```no_run
//! use conservator::{PooledConnection, Migrator};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let pool = PooledConnection::from_url("postgres://user:pass@localhost/db")?;
//! let mut conn = pool.get().await?;
//!
//! // Run migrations from ./migrations directory
//! Migrator::from_path("./migrations")?
//!     .run(&mut conn)
//!     .await?;
//! # Ok(())
//! # }
//! ```

use crate::{Connection, Creatable, Domain, Error, Executor, IntoValue, Value};
use conservator_macro::{Creatable as DeriveCreatable, Domain as DeriveDomain};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

// ============================================================================
// MigrationRecord - Using conservator's own ORM with #[derive(Domain)]!
// ============================================================================

/// Internal: Migration record stored in database
///
/// This is a showcase of conservator's ORM - we eat our own dog food!
#[derive(Debug, DeriveDomain)]
#[domain(table = "_conservator_migrations")]
#[allow(dead_code)]
struct MigrationRecord {
    #[domain(primary_key)]
    version: i64,
    description: String,
    checksum: Vec<u8>,
    success: bool,
    execution_time_ms: Option<i64>,
}

/// For inserting new migration records
#[derive(Debug, DeriveCreatable)]
struct CreateMigrationRecord {
    version: i64,
    description: String,
    checksum: Vec<u8>,
    success: bool,
}

// ============================================================================
// Public API
// ============================================================================

/// A single database migration
#[derive(Debug, Clone)]
pub struct Migration {
    /// Version number (extracted from filename)
    pub version: i64,
    /// Description (extracted from filename)
    pub description: String,
    /// SQL content
    pub sql: String,
    /// SHA256 checksum of SQL content
    pub checksum: Vec<u8>,
}

impl Migration {
    /// Create a new migration
    pub fn new(version: i64, description: impl Into<String>, sql: impl Into<String>) -> Self {
        let sql = sql.into();
        let checksum = Sha256::digest(sql.as_bytes()).to_vec();
        Self {
            version,
            description: description.into(),
            sql,
            checksum,
        }
    }
}

/// Applied migration record from database
#[derive(Debug)]
pub struct AppliedMigration {
    pub version: i64,
    pub checksum: Vec<u8>,
}

/// Migration error types
#[derive(Debug, thiserror::Error)]
pub enum MigrateError {
    #[error("Failed to read migration directory: {0}")]
    ReadDir(#[from] std::io::Error),

    #[error("Invalid migration filename: {0}")]
    InvalidFilename(String),

    #[error("Database error: {0}")]
    Database(#[from] Error),

    #[error("Migration {0} checksum mismatch - migration file was modified after being applied")]
    ChecksumMismatch(i64),

    #[error(
        "Migration {0} is in dirty state - previous migration failed, manual intervention required"
    )]
    Dirty(i64),

    #[error("Migration {0} was applied but is missing from source")]
    MissingSource(i64),
}

/// Database migrator
#[derive(Debug)]
pub struct Migrator {
    migrations: Vec<Migration>,
    /// Whether to use advisory locks (default: true)
    pub locking: bool,
    /// Whether to ignore missing migrations in source (default: false)
    pub ignore_missing: bool,
}

impl Migrator {
    /// Create a new migrator with no migrations
    pub fn new() -> Self {
        Self {
            migrations: Vec::new(),
            locking: true,
            ignore_missing: false,
        }
    }

    /// Load migrations from a directory
    ///
    /// Reads all files matching `<VERSION>_<DESCRIPTION>.sql` pattern.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use conservator::Migrator;
    ///
    /// let migrator = Migrator::from_path("./migrations")?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Self, MigrateError> {
        let path = path.as_ref();
        let mut migrations = Vec::new();

        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let file_path = entry.path();

            // Skip non-files
            if !file_path.is_file() {
                continue;
            }

            let file_name = match file_path.file_name().and_then(|n| n.to_str()) {
                Some(name) => name,
                None => continue,
            };

            // Skip non-SQL files
            if !file_name.ends_with(".sql") {
                continue;
            }

            // Skip down migrations (for future undo support)
            if file_name.contains(".down.") {
                continue;
            }

            // Parse filename: <VERSION>_<DESCRIPTION>.sql
            let parts: Vec<&str> = file_name.splitn(2, '_').collect();
            if parts.len() != 2 {
                continue; // Skip files that don't match pattern
            }

            let version: i64 = parts[0].parse().map_err(|_| {
                MigrateError::InvalidFilename(format!(
                    "cannot parse version from '{}', expected format: <VERSION>_<DESCRIPTION>.sql",
                    file_name
                ))
            })?;

            let description = parts[1]
                .trim_end_matches(".sql")
                .trim_end_matches(".up")
                .replace('_', " ");

            let sql = fs::read_to_string(&file_path)?;
            migrations.push(Migration::new(version, description, sql));
        }

        // Sort by version
        migrations.sort_by_key(|m| m.version);

        Ok(Self {
            migrations,
            locking: true,
            ignore_missing: false,
        })
    }

    /// Add a migration programmatically
    pub fn add_migration(&mut self, migration: Migration) -> &mut Self {
        self.migrations.push(migration);
        self.migrations.sort_by_key(|m| m.version);
        self
    }

    /// Set whether to use advisory locks
    pub fn set_locking(&mut self, locking: bool) -> &mut Self {
        self.locking = locking;
        self
    }

    /// Set whether to ignore missing migrations in source
    pub fn set_ignore_missing(&mut self, ignore: bool) -> &mut Self {
        self.ignore_missing = ignore;
        self
    }

    /// Get all migrations
    pub fn migrations(&self) -> &[Migration] {
        &self.migrations
    }

    /// Run all pending migrations
    ///
    /// This will:
    /// 1. Acquire an advisory lock (if enabled)
    /// 2. Create the migrations table if needed
    /// 3. Check for dirty state
    /// 4. Validate checksums of applied migrations
    /// 5. Apply pending migrations in a transaction
    /// 6. Release the lock
    pub async fn run(&self, conn: &mut Connection) -> Result<MigrateReport, MigrateError> {
        let mut report = MigrateReport::default();

        // Acquire lock
        if self.locking {
            self.lock(conn).await?;
        }

        let result = self.run_internal(conn, &mut report).await;

        // Always release lock
        if self.locking {
            let _ = self.unlock(conn).await;
        }

        result?;
        Ok(report)
    }

    async fn run_internal(
        &self,
        conn: &mut Connection,
        report: &mut MigrateReport,
    ) -> Result<(), MigrateError> {
        // Ensure migrations table exists
        self.ensure_migrations_table(conn).await?;

        // Check for dirty state using ORM
        let dirty = MigrationRecord::select()
            .filter(MigrationRecord::COLUMNS.success.eq(false))
            .order_by(MigrationRecord::COLUMNS.version)
            .limit(1)
            .optional(conn)
            .await?;

        if let Some(dirty_record) = dirty {
            return Err(MigrateError::Dirty(dirty_record.version));
        }

        // Get applied migrations using ORM
        let applied_records: Vec<MigrationRecord> = MigrationRecord::select()
            .filter(MigrationRecord::COLUMNS.success.eq(true))
            .order_by(MigrationRecord::COLUMNS.version)
            .all(conn)
            .await?;

        let applied_map: std::collections::HashMap<i64, Vec<u8>> = applied_records
            .into_iter()
            .map(|r| (r.version, r.checksum))
            .collect();

        // Validate checksums and check for missing
        if !self.ignore_missing {
            let source_versions: std::collections::HashSet<i64> =
                self.migrations.iter().map(|m| m.version).collect();
            for version in applied_map.keys() {
                if !source_versions.contains(version) {
                    return Err(MigrateError::MissingSource(*version));
                }
            }
        }

        // Apply pending migrations
        for migration in &self.migrations {
            if let Some(applied_checksum) = applied_map.get(&migration.version) {
                // Already applied - verify checksum
                if *applied_checksum != migration.checksum {
                    return Err(MigrateError::ChecksumMismatch(migration.version));
                }
                report.skipped += 1;
            } else {
                // Apply migration
                let duration = self.apply_migration(conn, migration).await?;
                report.applied.push(AppliedInfo {
                    version: migration.version,
                    description: migration.description.clone(),
                    duration,
                });
            }
        }

        Ok(())
    }

    async fn ensure_migrations_table(&self, conn: &mut Connection) -> Result<(), MigrateError> {
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS _conservator_migrations (
                version BIGINT PRIMARY KEY,
                description TEXT NOT NULL,
                checksum BYTEA NOT NULL,
                success BOOLEAN NOT NULL DEFAULT TRUE,
                applied_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                execution_time_ms BIGINT
            )
            "#,
            &[],
        )
        .await?;

        Ok(())
    }

    async fn apply_migration(
        &self,
        conn: &mut Connection,
        migration: &Migration,
    ) -> Result<std::time::Duration, MigrateError> {
        let start = std::time::Instant::now();

        // Start transaction
        let tx = conn.begin().await?;

        // Insert migration record as dirty (success=FALSE) using Creatable
        let _pk: i64 = CreateMigrationRecord {
            version: migration.version,
            description: migration.description.clone(),
            checksum: migration.checksum.clone(),
            success: false, // dirty state
        }
        .insert::<MigrationRecord>()
        .returning_pk(&tx)
        .await?;

        // Execute migration SQL
        tx.batch_execute(&migration.sql).await?;

        // Mark as success using UpdateBuilder
        let elapsed_ms = start.elapsed().as_millis() as i64;
        MigrationRecord::update()
            .set(MigrationRecord::COLUMNS.success, true)
            .set(MigrationRecord::COLUMNS.execution_time_ms, Some(elapsed_ms))
            .filter(MigrationRecord::COLUMNS.version.eq(migration.version))
            .execute(&tx)
            .await?;

        tx.commit().await?;

        Ok(start.elapsed())
    }

    async fn lock(&self, conn: &mut Connection) -> Result<(), MigrateError> {
        // Use a fixed lock ID for migrations
        let lock_id: i64 = 0x3d32ad9e * 0x636f6e73; // "conservator" hash
        let values: Vec<Value> = vec![lock_id.into_value()];
        let params: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> =
            values.iter().map(|v| v.as_param()).collect();

        conn.execute("SELECT pg_advisory_lock($1)", &params).await?;
        Ok(())
    }

    async fn unlock(&self, conn: &mut Connection) -> Result<(), MigrateError> {
        let lock_id: i64 = 0x3d32ad9e * 0x636f6e73;
        let values: Vec<Value> = vec![lock_id.into_value()];
        let params: Vec<&(dyn tokio_postgres::types::ToSql + Sync)> =
            values.iter().map(|v| v.as_param()).collect();

        conn.execute("SELECT pg_advisory_unlock($1)", &params)
            .await?;
        Ok(())
    }
}

impl Default for Migrator {
    fn default() -> Self {
        Self::new()
    }
}

/// Report of migration run
#[derive(Debug, Default)]
pub struct MigrateReport {
    /// Number of migrations skipped (already applied)
    pub skipped: usize,
    /// Applied migrations with details
    pub applied: Vec<AppliedInfo>,
}

impl MigrateReport {
    /// Check if any migrations were applied
    pub fn has_applied(&self) -> bool {
        !self.applied.is_empty()
    }

    /// Total number of migrations processed
    pub fn total(&self) -> usize {
        self.skipped + self.applied.len()
    }
}

/// Information about an applied migration
#[derive(Debug)]
pub struct AppliedInfo {
    pub version: i64,
    pub description: String,
    pub duration: std::time::Duration,
}

impl std::fmt::Display for MigrateReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.applied.is_empty() {
            write!(f, "No pending migrations")?;
        } else {
            writeln!(f, "Applied {} migration(s):", self.applied.len())?;
            for info in &self.applied {
                writeln!(
                    f,
                    "  {} - {} ({:.2?})",
                    info.version, info.description, info.duration
                )?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migration_checksum() {
        let m1 = Migration::new(1, "test", "CREATE TABLE foo (id INT)");
        let m2 = Migration::new(1, "test", "CREATE TABLE foo (id INT)");
        let m3 = Migration::new(1, "test", "CREATE TABLE bar (id INT)");

        assert_eq!(m1.checksum, m2.checksum);
        assert_ne!(m1.checksum, m3.checksum);
    }

    #[test]
    fn test_migrator_sorting() {
        let mut migrator = Migrator::new();
        migrator.add_migration(Migration::new(3, "third", "SELECT 3"));
        migrator.add_migration(Migration::new(1, "first", "SELECT 1"));
        migrator.add_migration(Migration::new(2, "second", "SELECT 2"));

        assert_eq!(migrator.migrations[0].version, 1);
        assert_eq!(migrator.migrations[1].version, 2);
        assert_eq!(migrator.migrations[2].version, 3);
    }
}
