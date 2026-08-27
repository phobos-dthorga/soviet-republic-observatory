use std::path::{Path, PathBuf};

use rusqlite::{OptionalExtension, params};

use super::ObservatoryStorage;
use crate::error::ObservatoryError;

impl ObservatoryStorage {
    pub fn set_setting(&self, key: &str, value: &Path) -> Result<(), ObservatoryError> {
        self.set_text_setting(key, &value.to_string_lossy())
    }

    pub fn set_bool_setting(&self, key: &str, value: bool) -> Result<(), ObservatoryError> {
        self.set_text_setting(key, if value { "true" } else { "false" })
    }

    fn set_text_setting(&self, key: &str, value: &str) -> Result<(), ObservatoryError> {
        let connection = self.connect()?;
        connection.execute(
            r#"INSERT INTO private_settings(setting_key, setting_value) VALUES(?1, ?2)
               ON CONFLICT(setting_key) DO UPDATE SET setting_value = excluded.setting_value"#,
            params![key, value],
        )?;
        Ok(())
    }

    pub fn get_setting(&self, key: &str) -> Result<Option<PathBuf>, ObservatoryError> {
        Ok(self.get_text_setting(key)?.map(PathBuf::from))
    }

    pub fn get_bool_setting(&self, key: &str) -> Result<bool, ObservatoryError> {
        Ok(self
            .get_text_setting(key)?
            .is_some_and(|value| value == "true"))
    }

    fn get_text_setting(&self, key: &str) -> Result<Option<String>, ObservatoryError> {
        let connection = self.connect()?;
        let value = connection
            .query_row(
                "SELECT setting_value FROM private_settings WHERE setting_key = ?1",
                [key],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        Ok(value)
    }
}
