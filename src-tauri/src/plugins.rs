use serde::Deserialize;

/// Plugin manifest — matches the frontend PluginDef (without `component`).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginManifest {
    pub id: String,
    pub title: String,
    pub default_width: Option<f64>,
    pub default_height: Option<f64>,
    pub default_attach_enabled: Option<bool>,
    pub default_attach_remember: Option<bool>,
    pub default_whitelist: Option<Vec<String>>,
    #[allow(dead_code)]
    pub description: Option<String>,
    #[allow(dead_code)]
    pub icon: Option<String>,
    pub show_close_button: Option<bool>,
    pub show_collapse_button: Option<bool>,
    pub show_attach_button: Option<bool>,
}

// Include the generated manifest code
include!(concat!(env!("OUT_DIR"), "/plugin_manifests.rs"));

/// Load all plugin manifests (from compile-time embedded data).
pub fn load_manifests() -> Vec<PluginManifest> {
    embedded_plugin_manifests()
        .into_iter()
        .filter_map(|v| serde_json::from_value(v).ok())
        .collect()
}
