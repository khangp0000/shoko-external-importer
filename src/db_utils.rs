use crate::models::*;
use crate::schema::processed_files::dsl::*;
use anyhow::Context;
use anyhow::{anyhow, bail, Result};
use diesel::prelude::*;
use diesel::{
    insert_into,
    migration::MigrationVersion,
    r2d2::{ConnectionManager, Pool, PooledConnection},
    sqlite::Sqlite,
    SqliteConnection,
};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use std::path::Path;
use std::sync::{Arc, RwLock};
use std::sync::{Mutex, MutexGuard, RwLockReadGuard, RwLockWriteGuard};

const MIGRATIONS: EmbeddedMigrations = embed_migrations!("./migrations/");

pub struct FileProcessingSqliteDb {
    pool: Pool<ConnectionManager<SqliteConnection>>,
    db_lock: Arc<RwLock<()>>,
}

impl FileProcessingSqliteDb {
    pub fn new(pool: Pool<ConnectionManager<SqliteConnection>>) -> FileProcessingSqliteDb {
        FileProcessingSqliteDb {
            pool,
            db_lock: Arc::new(RwLock::new(())),
        }
    }

    pub fn create_from_file(db: &Path) -> Result<FileProcessingSqliteDb> {
        let manager = ConnectionManager::<SqliteConnection>::new(db.display().to_string());
        Pool::builder()
            .max_size(30)
            .min_idle(Some(5))
            .test_on_check_out(true)
            .build(manager)
            .map(FileProcessingSqliteDb::new)
            .map_err(anyhow::Error::from)
    }

    pub fn create_connection(&self) -> Result<FileProcessingSqliteDbConnection> {
        Ok(FileProcessingSqliteDbConnection {
            lock: self.db_lock.clone(),
            connection: Mutex::new(self.pool.get().map_err(|e| anyhow!(e))?),
        })
    }
}

pub struct FileProcessingSqliteDbConnection {
    lock: Arc<RwLock<()>>,
    connection: Mutex<PooledConnection<ConnectionManager<SqliteConnection>>>,
}

type ReadLockGuard<'a> = RwLockReadGuard<'a, ()>;
type WriteLockGuard<'a> = RwLockWriteGuard<'a, ()>;
type PooledConnectionLockGuard<'a> =
    MutexGuard<'a, PooledConnection<ConnectionManager<SqliteConnection>>>;
impl FileProcessingSqliteDbConnection {
    fn lock_write_connection(&self) -> Result<(WriteLockGuard, PooledConnectionLockGuard)> {
        Ok((
            self.lock.write().map_err(|e| anyhow!(e.to_string()))?,
            self.connection.lock().map_err(|e| anyhow!(e.to_string()))?,
        ))
    }

    fn lock_read_connection(&self) -> Result<(ReadLockGuard, PooledConnectionLockGuard)> {
        Ok((
            self.lock.read().map_err(|e| anyhow!(e.to_string()))?,
            self.connection.lock().map_err(|e| anyhow!(e.to_string()))?,
        ))
    }

    pub fn run_migrations(&self) -> Result<()> {
        let (_db_lock, mut connection_lock) = self.lock_write_connection()?;
        run_migrations_with_connection(&mut *connection_lock)?;
        Ok(())
    }

    pub fn add_processed_file(&self, path: &Path) -> Result<()> {
        let (_db_lock, mut connection_lock) = self.lock_write_connection()?;

        let processed_file_entry = NewProcessedFile {
            file_path: path.display().to_string(),
        };

        match insert_into(processed_files)
            .values(&processed_file_entry)
            .execute(&mut *connection_lock)
        {
            Ok(count) => match count {
                1 => Ok(()),
                invalid_count => bail!(
                    "SQL return invalid count when add a processed path: {}",
                    invalid_count
                ),
            },
            Err(e) => Err(anyhow!(e)),
        }
        .with_context(|| {
            format!(
                "Failed to add processed entry to db: {}",
                &processed_file_entry.file_path
            )
        })
    }

    pub fn is_path_processed(&self, path: &Path) -> Result<bool> {
        let (_db_lock, mut connection_lock) = self.lock_read_connection()?;

        match processed_files
            .filter(file_path.eq(path.display().to_string()))
            .first::<ProcessedFile>(&mut *connection_lock)
        {
            Ok(_) => Ok(true),
            Err(diesel::NotFound) => Ok(false),
            Err(e) => Err(anyhow::Error::from(e))
                .with_context(|| format!("Error checking file entry exists: {}", &path.display())),
        }
    }
}

fn run_migrations_with_connection(
    connection: &mut impl MigrationHarness<Sqlite>,
) -> Result<Vec<MigrationVersion>> {
    connection
        .run_pending_migrations(MIGRATIONS)
        .map_err(|e| anyhow!(e))
}
