//! Database operations using rusqlite.
//!
//! T009: Implement Database struct with connection and migration

use crate::storage::schema::{CURRENT_VERSION, SCHEMA, SCHEMA_VERSION_TABLE};
use rusqlite::{Connection, Result as SqliteResult};
use std::path::PathBuf;
use thiserror::Error;

/// Database wrapper for SQLite operations.
pub struct Database {
    conn: Connection,
}

impl Database {
    /// Open or create a database at the given path.
    pub fn open(path: &PathBuf) -> Result<Self, DatabaseError> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| DatabaseError::IoError(e.to_string()))?;
        }

        let conn =
            Connection::open(path).map_err(|e| DatabaseError::ConnectionFailed(e.to_string()))?;

        let db = Self { conn };
        db.initialize()?;

        Ok(db)
    }

    /// Open an in-memory database (for testing).
    pub fn open_in_memory() -> Result<Self, DatabaseError> {
        let conn =
            Connection::open_in_memory().map_err(|e| DatabaseError::ConnectionFailed(e.to_string()))?;

        let db = Self { conn };
        db.initialize()?;

        Ok(db)
    }

    /// Initialize the database schema.
    fn initialize(&self) -> Result<(), DatabaseError> {
        // Create schema version table
        self.conn
            .execute_batch(SCHEMA_VERSION_TABLE)
            .map_err(|e| DatabaseError::MigrationFailed(e.to_string()))?;

        // Check current version
        let current_version = self.get_schema_version()?;

        if current_version < CURRENT_VERSION {
            self.migrate(current_version)?;
        }

        Ok(())
    }

    /// Get the current schema version.
    fn get_schema_version(&self) -> Result<i32, DatabaseError> {
        let result: SqliteResult<i32> = self.conn.query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_version",
            [],
            |row| row.get(0),
        );

        match result {
            Ok(version) => Ok(version),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(0),
            Err(e) => Err(DatabaseError::QueryFailed(e.to_string())),
        }
    }

    /// Run database migrations.
    fn migrate(&self, from_version: i32) -> Result<(), DatabaseError> {
        if from_version < 1 {
            // Initial schema
            self.conn
                .execute_batch(SCHEMA)
                .map_err(|e| DatabaseError::MigrationFailed(e.to_string()))?;

            // Record version
            self.conn
                .execute(
                    "INSERT INTO schema_version (version, applied_at) VALUES (?, datetime('now'))",
                    [CURRENT_VERSION],
                )
                .map_err(|e| DatabaseError::MigrationFailed(e.to_string()))?;

            tracing::info!("Database migrated to version {}", CURRENT_VERSION);
        }

        // Future migrations would go here:
        // if from_version < 2 { ... }

        Ok(())
    }

    /// Get a reference to the underlying connection.
    pub fn connection(&self) -> &Connection {
        &self.conn
    }

    /// Execute a query and return the number of rows affected.
    pub fn execute(&self, sql: &str, params: &[&dyn rusqlite::ToSql]) -> Result<usize, DatabaseError> {
        self.conn
            .execute(sql, params)
            .map_err(|e| DatabaseError::QueryFailed(e.to_string()))
    }

    /// Begin a transaction.
    pub fn transaction(&mut self) -> Result<rusqlite::Transaction<'_>, DatabaseError> {
        self.conn
            .transaction()
            .map_err(|e| DatabaseError::TransactionFailed(e.to_string()))
    }
}

/// Database errors.
#[derive(Debug, Error)]
pub enum DatabaseError {
    #[error("Failed to connect to database: {0}")]
    ConnectionFailed(String),

    #[error("IO error: {0}")]
    IoError(String),

    #[error("Migration failed: {0}")]
    MigrationFailed(String),

    #[error("Query failed: {0}")]
    QueryFailed(String),

    #[error("Transaction failed: {0}")]
    TransactionFailed(String),

    #[error("Record not found: {0}")]
    NotFound(String),

    #[error("Constraint violation: {0}")]
    ConstraintViolation(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_in_memory_database() {
        let db = Database::open_in_memory().expect("Failed to create database");
        let version = db.get_schema_version().expect("Failed to get version");
        assert_eq!(version, CURRENT_VERSION);
    }

    #[test]
    fn test_tables_created() {
        let db = Database::open_in_memory().expect("Failed to create database");

        // Check that tables exist
        let tables: Vec<String> = db
            .conn
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .filter_map(|r| r.ok())
            .collect();

        assert!(tables.contains(&"users".to_string()));
        assert!(tables.contains(&"sensors".to_string()));
        assert!(tables.contains(&"workouts".to_string()));
        assert!(tables.contains(&"rides".to_string()));
        assert!(tables.contains(&"ride_samples".to_string()));
        assert!(tables.contains(&"autosave".to_string()));
    }
}
