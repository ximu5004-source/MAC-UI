use seelen_core::state::{AppConfig, AppsConfigurationList};

use crate::{
    error::Result,
    state::application::{BUNDLED_SETTINGS_BY_APP, FULL_STATE},
    utils::constants::SEELEN_COMMON,
    windows_api::window::Window,
};

use super::AppSettings;

impl Window {
    pub fn get_app_config(&self) -> Result<Option<AppConfig>> {
        let path = self.process().program_path()?;

        let exe = path.file_name().ok_or("Invalid path")?.to_string_lossy();
        let path = path.to_string_lossy();
        let title = self.title();
        let class = self.class();

        if let Some(app) = FULL_STATE
            .load()
            .settings
            .by_app
            .search(&title, &class, &exe, &path)
        {
            return Ok(Some(app.clone()));
        }

        Ok(BUNDLED_SETTINGS_BY_APP
            .search(&title, &class, &exe, &path)
            .cloned())
    }
}

impl AppSettings {
    pub(super) fn load_bundled_settings_by_app() -> Result<AppsConfigurationList> {
        let apps_templates_path = SEELEN_COMMON.bundled_app_configs_path();
        let mut configs = AppsConfigurationList::default();

        for entry in apps_templates_path.read_dir()?.flatten() {
            let file = std::fs::File::open(entry.path())?;
            let mut apps: Vec<AppConfig> = serde_yaml::from_reader(&file)?;
            for app in apps.iter_mut() {
                app.is_bundled = true;
            }
            configs.extend(apps);
        }

        configs.prepare();
        Ok(configs)
    }
}
