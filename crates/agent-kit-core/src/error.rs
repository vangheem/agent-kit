#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ConfigError {
    #[error("missing environment variable {0}")]
    Missing(&'static str),
    #[error("APP_SECRET must be at least 32 bytes")]
    SecretTooShort,
    #[error("invalid {name}: {value}")]
    Invalid { name: &'static str, value: String },
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum CryptoError {
    #[error("encryption failed")]
    Encrypt,
    #[error("decryption failed")]
    Decrypt,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum BlobError {
    #[error("blob exceeds 10 MiB")]
    TooLarge,
    #[error("blob not found")]
    NotFound,
    #[error("blob storage error: {message}")]
    Internal { message: String },
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ActionError {
    #[error("not_found")]
    NotFound,
    #[error("forbidden")]
    Forbidden,
    #[error("confirmation_required")]
    ConfirmationRequired,
    #[error("provider_not_configured: {provider}")]
    ProviderNotConfigured { provider: String },
    #[error("invalid_input: {message}")]
    InvalidInput { message: String },
    #[error("provider_error: {message}")]
    Provider { message: String },
    #[error("live_trading_disabled")]
    LiveTradingDisabled,
    #[error("internal: {message}")]
    Internal { message: String },
}

impl ActionError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::NotFound => "not_found",
            Self::Forbidden => "forbidden",
            Self::ConfirmationRequired => "confirmation_required",
            Self::ProviderNotConfigured { .. } => "provider_not_configured",
            Self::InvalidInput { .. } => "invalid_input",
            Self::Provider { .. } => "provider_error",
            Self::LiveTradingDisabled => "live_trading_disabled",
            Self::Internal { .. } => "internal",
        }
    }
}
