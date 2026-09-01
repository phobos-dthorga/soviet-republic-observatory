use std::path::{Path, PathBuf};

use rusqlite::{OptionalExtension, params};

use super::ObservatoryStorage;
use crate::error::ObservatoryError;
use crate::model::{
    APPLICATION_PREFERENCES_SCHEMA_VERSION, ApplicationPreferences, ApplicationPreferencesDraft,
    BackgroundWorkPriority, MAX_STORAGE_PATIENCE_SECONDS, MIN_STORAGE_PATIENCE_SECONDS,
    MotionPreference, StoragePatiencePreset, WordingMode,
};

const AUTOMATIC_OBSERVATION_KEY: &str = "automatic_observation_enabled";
const STORAGE_PATIENCE_PRESET_KEY: &str = "storage_patience_preset";
const CUSTOM_STORAGE_PATIENCE_SECONDS_KEY: &str = "custom_storage_patience_seconds";
const BACKGROUND_WORK_PRIORITY_KEY: &str = "background_work_priority";
const TEXT_SCALE_PERCENT_KEY: &str = "interface_text_scale_percent";
const MOTION_PREFERENCE_KEY: &str = "motion_preference";
const WORDING_MODE_KEY: &str = "wording_mode";

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

    pub fn load_application_preferences(&self) -> Result<ApplicationPreferences, ObservatoryError> {
        let connection = self.connect()?;
        let automatic_observation_enabled = text_setting(&connection, AUTOMATIC_OBSERVATION_KEY)?
            .is_some_and(|value| value == "true");
        let preset = text_setting(&connection, STORAGE_PATIENCE_PRESET_KEY)?
            .as_deref()
            .and_then(parse_storage_patience_preset)
            .unwrap_or_default();
        let custom_seconds = text_setting(&connection, CUSTOM_STORAGE_PATIENCE_SECONDS_KEY)?
            .and_then(|value| value.parse::<u16>().ok())
            .filter(|value| {
                (MIN_STORAGE_PATIENCE_SECONDS..=MAX_STORAGE_PATIENCE_SECONDS).contains(value)
            });
        let background_work_priority = text_setting(&connection, BACKGROUND_WORK_PRIORITY_KEY)?
            .as_deref()
            .and_then(parse_background_work_priority)
            .unwrap_or_default();
        let text_scale_percent = text_setting(&connection, TEXT_SCALE_PERCENT_KEY)?
            .and_then(|value| value.parse::<u16>().ok())
            .filter(|value| matches!(value, 100 | 125 | 150 | 175 | 200))
            .unwrap_or(100);
        let motion_preference = text_setting(&connection, MOTION_PREFERENCE_KEY)?
            .as_deref()
            .and_then(parse_motion_preference)
            .unwrap_or_default();
        let wording_mode = text_setting(&connection, WORDING_MODE_KEY)?
            .as_deref()
            .and_then(parse_wording_mode)
            .unwrap_or_default();

        Ok(application_preferences(
            preset,
            custom_seconds,
            background_work_priority,
            text_scale_percent,
            motion_preference,
            wording_mode,
            automatic_observation_enabled,
        ))
    }

    pub fn save_application_preferences(
        &self,
        draft: &ApplicationPreferencesDraft,
    ) -> Result<ApplicationPreferences, ObservatoryError> {
        let custom_seconds = validate_application_preferences(draft)?;
        let mut connection = self.connect()?;
        let transaction = connection.transaction()?;
        for (key, value) in [
            (
                AUTOMATIC_OBSERVATION_KEY,
                if draft.automatic_observation_enabled {
                    "true".to_owned()
                } else {
                    "false".to_owned()
                },
            ),
            (
                STORAGE_PATIENCE_PRESET_KEY,
                storage_patience_preset_name(draft.storage_patience_preset).to_owned(),
            ),
            (
                BACKGROUND_WORK_PRIORITY_KEY,
                background_work_priority_name(draft.background_work_priority).to_owned(),
            ),
            (TEXT_SCALE_PERCENT_KEY, draft.text_scale_percent.to_string()),
            (
                MOTION_PREFERENCE_KEY,
                motion_preference_name(draft.motion_preference).to_owned(),
            ),
            (
                WORDING_MODE_KEY,
                wording_mode_name(draft.wording_mode).to_owned(),
            ),
        ] {
            set_text_setting_in(&transaction, key, &value)?;
        }
        if let Some(seconds) = custom_seconds {
            set_text_setting_in(
                &transaction,
                CUSTOM_STORAGE_PATIENCE_SECONDS_KEY,
                &seconds.to_string(),
            )?;
        } else {
            transaction.execute(
                "DELETE FROM private_settings WHERE setting_key = ?1",
                [CUSTOM_STORAGE_PATIENCE_SECONDS_KEY],
            )?;
        }
        transaction.commit()?;

        Ok(application_preferences(
            draft.storage_patience_preset,
            custom_seconds,
            draft.background_work_priority,
            draft.text_scale_percent,
            draft.motion_preference,
            draft.wording_mode,
            draft.automatic_observation_enabled,
        ))
    }

    pub fn reset_application_preferences(
        &self,
    ) -> Result<ApplicationPreferences, ObservatoryError> {
        let automatic_observation_enabled = self.get_bool_setting(AUTOMATIC_OBSERVATION_KEY)?;
        self.save_application_preferences(&ApplicationPreferencesDraft {
            storage_patience_preset: StoragePatiencePreset::Balanced,
            custom_storage_patience_seconds: None,
            background_work_priority: BackgroundWorkPriority::Gentle,
            text_scale_percent: 100,
            motion_preference: MotionPreference::System,
            wording_mode: WordingMode::PlayerFriendly,
            automatic_observation_enabled,
        })
    }
}

fn text_setting(
    connection: &rusqlite::Connection,
    key: &str,
) -> Result<Option<String>, ObservatoryError> {
    Ok(connection
        .query_row(
            "SELECT setting_value FROM private_settings WHERE setting_key = ?1",
            [key],
            |row| row.get::<_, String>(0),
        )
        .optional()?)
}

fn set_text_setting_in(
    transaction: &rusqlite::Transaction<'_>,
    key: &str,
    value: &str,
) -> Result<(), ObservatoryError> {
    transaction.execute(
        r#"INSERT INTO private_settings(setting_key, setting_value) VALUES(?1, ?2)
           ON CONFLICT(setting_key) DO UPDATE SET setting_value = excluded.setting_value"#,
        params![key, value],
    )?;
    Ok(())
}

fn validate_application_preferences(
    draft: &ApplicationPreferencesDraft,
) -> Result<Option<u16>, ObservatoryError> {
    if !matches!(draft.text_scale_percent, 100 | 125 | 150 | 175 | 200) {
        return Err(ObservatoryError::InvalidApplicationPreferences);
    }
    match draft.storage_patience_preset {
        StoragePatiencePreset::Custom => draft
            .custom_storage_patience_seconds
            .filter(|value| {
                (MIN_STORAGE_PATIENCE_SECONDS..=MAX_STORAGE_PATIENCE_SECONDS).contains(value)
            })
            .map(Some)
            .ok_or(ObservatoryError::InvalidApplicationPreferences),
        _ if draft.custom_storage_patience_seconds.is_some() => {
            Err(ObservatoryError::InvalidApplicationPreferences)
        }
        _ => Ok(None),
    }
}

fn application_preferences(
    preset: StoragePatiencePreset,
    custom_seconds: Option<u16>,
    background_work_priority: BackgroundWorkPriority,
    text_scale_percent: u16,
    motion_preference: MotionPreference,
    wording_mode: WordingMode,
    automatic_observation_enabled: bool,
) -> ApplicationPreferences {
    let effective_storage_patience_seconds = match preset {
        StoragePatiencePreset::Short => 15,
        StoragePatiencePreset::Balanced => 60,
        StoragePatiencePreset::Patient => 180,
        StoragePatiencePreset::Custom => custom_seconds.unwrap_or(60),
    };
    ApplicationPreferences {
        schema_version: APPLICATION_PREFERENCES_SCHEMA_VERSION,
        storage_patience_preset: preset,
        custom_storage_patience_seconds: custom_seconds,
        effective_storage_patience_seconds,
        background_work_priority,
        text_scale_percent,
        motion_preference,
        wording_mode,
        automatic_observation_enabled,
    }
}

fn parse_storage_patience_preset(value: &str) -> Option<StoragePatiencePreset> {
    match value {
        "short" => Some(StoragePatiencePreset::Short),
        "balanced" => Some(StoragePatiencePreset::Balanced),
        "patient" => Some(StoragePatiencePreset::Patient),
        "custom" => Some(StoragePatiencePreset::Custom),
        _ => None,
    }
}

fn storage_patience_preset_name(value: StoragePatiencePreset) -> &'static str {
    match value {
        StoragePatiencePreset::Short => "short",
        StoragePatiencePreset::Balanced => "balanced",
        StoragePatiencePreset::Patient => "patient",
        StoragePatiencePreset::Custom => "custom",
    }
}

fn parse_background_work_priority(value: &str) -> Option<BackgroundWorkPriority> {
    match value {
        "gentle" => Some(BackgroundWorkPriority::Gentle),
        "balanced" => Some(BackgroundWorkPriority::Balanced),
        "finish_sooner" => Some(BackgroundWorkPriority::FinishSooner),
        _ => None,
    }
}

fn background_work_priority_name(value: BackgroundWorkPriority) -> &'static str {
    match value {
        BackgroundWorkPriority::Gentle => "gentle",
        BackgroundWorkPriority::Balanced => "balanced",
        BackgroundWorkPriority::FinishSooner => "finish_sooner",
    }
}

fn parse_motion_preference(value: &str) -> Option<MotionPreference> {
    match value {
        "system" => Some(MotionPreference::System),
        "reduced" => Some(MotionPreference::Reduced),
        _ => None,
    }
}

fn motion_preference_name(value: MotionPreference) -> &'static str {
    match value {
        MotionPreference::System => "system",
        MotionPreference::Reduced => "reduced",
    }
}

fn parse_wording_mode(value: &str) -> Option<WordingMode> {
    match value {
        "player_friendly" => Some(WordingMode::PlayerFriendly),
        "technical" => Some(WordingMode::Technical),
        _ => None,
    }
}

fn wording_mode_name(value: WordingMode) -> &'static str {
    match value {
        WordingMode::PlayerFriendly => "player_friendly",
        WordingMode::Technical => "technical",
    }
}
