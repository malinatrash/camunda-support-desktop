use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CamundaConnection {
    pub id: String,
    pub name: String,
    pub rest_url: String,
    pub cockpit_url: String,
    pub environment: Environment,
    #[serde(default)]
    pub engine_version: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Environment {
    Production,
    #[default]
    Test,
    Development,
    Local,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionDraft {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub rest_url: String,
    #[serde(default)]
    pub cockpit_url: String,
    #[serde(default = "default_environment")]
    pub environment: Environment,
}

fn default_environment() -> Environment {
    Environment::Test
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionValidation {
    pub name_error: Option<String>,
    pub rest_url_error: Option<String>,
    pub cockpit_url_error: Option<String>,
}

impl ConnectionValidation {
    pub fn is_valid(&self) -> bool {
        self.name_error.is_none() && self.rest_url_error.is_none() && self.cockpit_url_error.is_none()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionTestSuccess {
    pub engine_version: Option<String>,
}
