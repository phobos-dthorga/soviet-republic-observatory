use std::path::{Path, PathBuf};

use rusqlite::{OptionalExtension, params};

use super::ObservatoryStorage;
use crate::error::ObservatoryError;

impl ObservatoryStorage {
    pub fn set_setting(&self, key: &str, value: &Path) -> Result<(), ObservatoryError> {
        let connection = self.connect()?;
        connection.execute(
            r#"INSERT INTO private_settings(setting_key, setting_value) VALUES(?1, ?2)
               ON CONFLICT(setting_key) DO UPDATE SET setting_value = excluded.setting_value"#,
            params![key, value.to_string_lossy()],
        )?;
        Ok(())
    }

    pub fn get_setting(&self, key: &str) -> Result<Option<PathBuf>, ObservatoryError> {
        let connection = self.connect()?;
        let value = connection
            .query_row(
                "SELECT setting_value FROM private_settings WHERE setting_key = ?1",
                [key],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        Ok(value.map(PathBuf::from))
    }
}
