use galvyn::contrib::settings::ApplicationSettings;
use galvyn::contrib::settings::RegisterError;
use galvyn::contrib::settings::SettingsHandle;
use galvyn::contrib::settings::SettingsStore;
use serde::Deserialize;
use serde::Serialize;

pub struct Settings {
    pub origin: SettingsHandle<String>,

    #[expect(unused, reason = "Dummy for illustration")]
    pub cors: SettingsHandle<Cors>,
}

#[derive(Default, Serialize, Deserialize)]
pub struct Cors {
    pub allowed_origins: Vec<String>,
}

impl ApplicationSettings for Settings {
    fn init(store: &mut SettingsStore) -> Result<Self, RegisterError> {
        Ok(Self {
            origin: store.register("Settings.origin", || "https://blog.example.com".to_string())?,
            cors: store.register("Settings.cors", Default::default)?,
        })
    }
}
