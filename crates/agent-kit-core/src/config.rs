use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

pub use crate::error::ConfigError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlobBackendKind {
    Pg,
    Fs,
}

impl BlobBackendKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pg => "pg",
            Self::Fs => "fs",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "pg" => Some(Self::Pg),
            "fs" => Some(Self::Fs),
            _ => None,
        }
    }
}

impl std::fmt::Display for BlobBackendKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Clone)]
pub struct AppConfig {
    pub database_url: String,
    pub app_secret: String,
    pub blob_storage_type: BlobBackendKind,
    pub blob_fs_root: PathBuf,
    pub allow_mock: bool,
    pub session_ttl: Duration,
    pub cookie_secure: bool,
    pub bind: SocketAddr,
    pub cors_origin: String,
}

impl std::fmt::Debug for AppConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppConfig")
            .field("database_url", &"<redacted>")
            .field("app_secret", &"<redacted>")
            .field("blob_storage_type", &self.blob_storage_type)
            .field("blob_fs_root", &self.blob_fs_root)
            .field("allow_mock", &self.allow_mock)
            .field("session_ttl", &self.session_ttl)
            .field("cookie_secure", &self.cookie_secure)
            .field("bind", &self.bind)
            .field("cors_origin", &self.cors_origin)
            .finish()
    }
}

pub fn load_config() -> Result<AppConfig, ConfigError> {
    let database_url = required("DATABASE_URL")?;
    let app_secret = required("APP_SECRET")?;
    if app_secret.len() < 32 {
        return Err(ConfigError::SecretTooShort);
    }

    let blob_storage_type = match optional("BLOB_STORAGE_TYPE").as_deref() {
        None => BlobBackendKind::Pg,
        Some(value) => BlobBackendKind::parse(value).ok_or_else(|| ConfigError::Invalid {
            name: "BLOB_STORAGE_TYPE",
            value: value.to_owned(),
        })?,
    };

    let blob_fs_root = optional("BLOB_FS_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/var/lib/agent-kit/blobs"));

    let allow_mock = match optional("AGENT_KIT_ALLOW_MOCK").as_deref() {
        None => false,
        Some(value) => parse_bool("AGENT_KIT_ALLOW_MOCK", value)?,
    };

    let session_ttl_hours = match optional("SESSION_TTL_HOURS").as_deref() {
        None => 168,
        Some(value) => parse_u64("SESSION_TTL_HOURS", value)?,
    };
    let session_ttl =
        Duration::from_secs(session_ttl_hours.checked_mul(3600).ok_or_else(|| {
            ConfigError::Invalid {
                name: "SESSION_TTL_HOURS",
                value: session_ttl_hours.to_string(),
            }
        })?);

    let cookie_secure = match optional("SESSION_COOKIE_SECURE").as_deref() {
        None => false,
        Some(value) => parse_bool("SESSION_COOKIE_SECURE", value)?,
    };

    let bind = match optional("BIND_ADDR").as_deref() {
        None => "0.0.0.0:7010".parse().expect("default bind addr"),
        Some(value) => value.parse().map_err(|_| ConfigError::Invalid {
            name: "BIND_ADDR",
            value: value.to_owned(),
        })?,
    };

    let cors_origin = optional("CORS_ORIGIN").unwrap_or_else(|| "http://localhost:4210".to_owned());

    Ok(AppConfig {
        database_url,
        app_secret,
        blob_storage_type,
        blob_fs_root,
        allow_mock,
        session_ttl,
        cookie_secure,
        bind,
        cors_origin,
    })
}

fn required(name: &'static str) -> Result<String, ConfigError> {
    match std::env::var(name) {
        Ok(value) if !value.is_empty() => Ok(value),
        _ => Err(ConfigError::Missing(name)),
    }
}

fn optional(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|value| !value.is_empty())
}

fn parse_bool(name: &'static str, value: &str) -> Result<bool, ConfigError> {
    if value.eq_ignore_ascii_case("true") || value == "1" {
        Ok(true)
    } else if value.eq_ignore_ascii_case("false") || value == "0" {
        Ok(false)
    } else {
        Err(ConfigError::Invalid {
            name,
            value: value.to_owned(),
        })
    }
}

fn parse_u64(name: &'static str, value: &str) -> Result<u64, ConfigError> {
    value.parse().map_err(|_| ConfigError::Invalid {
        name,
        value: value.to_owned(),
    })
}
