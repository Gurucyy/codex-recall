use codex_vault_core::models::{AppSettings, ScanResult};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct AppState {
    scans: Arc<Mutex<HashMap<String, ScanResult>>>,
    settings: Arc<Mutex<AppSettings>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            scans: Arc::new(Mutex::new(HashMap::new())),
            settings: Arc::new(Mutex::new(AppSettings::default())),
        }
    }
}

impl AppState {
    pub fn insert_scan(&self, scan: ScanResult) {
        self.scans.lock().insert(scan.scan_id.clone(), scan);
    }

    pub fn get_scan(&self, scan_id: &str) -> Option<ScanResult> {
        self.scans.lock().get(scan_id).cloned()
    }

    pub fn settings(&self) -> AppSettings {
        self.settings.lock().clone()
    }

    pub fn update_settings(&self, settings: AppSettings) -> AppSettings {
        *self.settings.lock() = settings;
        self.settings()
    }
}
