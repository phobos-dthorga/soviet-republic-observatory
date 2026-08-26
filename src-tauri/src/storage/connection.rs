use std::time::Duration;

use rusqlite::Connection;

use super::ObservatoryStorage;
use crate::error::ObservatoryError;

impl ObservatoryStorage {
    pub(crate) fn connect(&self) -> Result<Connection, ObservatoryError> {
        let connection = Connection::open(&self.database_path)?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
        connection.pragma_update(None, "journal_mode", "WAL")?;
        connection.busy_timeout(Duration::from_secs(5))?;
        Ok(connection)
    }
}
