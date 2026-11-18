use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Unsupported(String),
    #[error("{0}")]
    Incompatible(String),
    #[error("Another switch is already running for {0}")]
    Busy(String),
    #[error("{0}")]
    ConfirmationRequired(String),
    #[error("{0}")]
    Verification(String),
    #[error("{0}")]
    Credential(String),
    #[error("{0}")]
    Platform(String),
    #[error("{0}")]
    Invalid(String),
    #[error("{0}")]
    Cancelled(String),
    #[error("{0}")]
    Database(String),
    #[error("{context}: {source}")]
    Io {
        context: String,
        #[source]
        source: std::io::Error,
    },
    #[error("Invalid data: {0}")]
    Json(#[from] serde_json::Error),
}

impl Error {
    pub fn code(&self) -> &'static str {
        match self {
            Error::NotFound(_) => "not_found",
            Error::Unsupported(_) => "unsupported",
            Error::Incompatible(_) => "incompatible",
            Error::Busy(_) => "busy",
            Error::ConfirmationRequired(_) => "confirmation_required",
            Error::Verification(_) => "verification_failed",
            Error::Credential(_) => "credential_error",
            Error::Platform(_) => "platform_error",
            Error::Invalid(_) => "invalid",
            Error::Cancelled(_) => "cancelled",
            Error::Database(_) => "database_error",
            Error::Io { .. } => "io_error",
            Error::Json(_) => "invalid_data",
        }
    }

    pub fn io(context: impl Into<String>, source: std::io::Error) -> Self {
        Error::Io { context: context.into(), source }
    }
}

#[derive(Debug, Clone, Serialize, ts_rs::TS)]
#[ts(export_to = "bindings.ts")]
pub struct ApiError {
    pub code: String,
    pub message: String,
}

impl Serialize for Error {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        ApiError { code: self.code().into(), message: self.to_string() }.serialize(s)
    }
}

pub type Result<T> = std::result::Result<T, Error>;
