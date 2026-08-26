use rusqlite::{Connection, params};

use super::now_ms;
use crate::error::ObservatoryError;

const MIGRATIONS: &[(i64, &str, &str)] = &[
    (
        1,
        "observation foundation",
        include_str!("../../migrations/0001_observations.sql"),
    ),
    (
        2,
        "branch-aware archive",
        include_str!("../../migrations/0002_branch_archive.sql"),
    ),
];

pub(crate) fn apply(connection: &mut Connection) -> Result<(), ObservatoryError> {
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (\
             version INTEGER PRIMARY KEY,\
             name TEXT NOT NULL,\
             applied_at_ms INTEGER NOT NULL\
         ) STRICT;",
    )?;

    for (version, name, sql) in MIGRATIONS {
        let applied = connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version = ?1)",
            [version],
            |row| row.get::<_, bool>(0),
        )?;
        if applied {
            continue;
        }
        let transaction = connection.transaction()?;
        transaction.execute_batch(sql)?;
        transaction.execute(
            "INSERT INTO schema_migrations(version, name, applied_at_ms) VALUES(?1, ?2, ?3)",
            params![version, name, now_ms()],
        )?;
        transaction.commit()?;
    }
    Ok(())
}
