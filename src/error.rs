use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use tracing::error;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("A provided Input was incorrect")]
    UserInputWrong,
    #[error("an error occurred with the database")]
    Sqlx(#[from] sqlx::Error),
    #[error("an internal server error occurred")]
    Anyhow(#[from] anyhow::Error),
    #[error("a rendering error occurred")]
    Sailfish(#[from] sailfish::runtime::RenderError),
}

pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::UserInputWrong => StatusCode::BAD_REQUEST,
            Self::Sqlx(_) | Self::Anyhow(_) | Self::Sailfish(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        }
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        match self {
            Self::UserInputWrong => error!("Got malformed input"),
            Self::Sqlx(ref e) => error!("SQLx error: {:?}", e),
            Self::Anyhow(ref e) => error!("Generic error: {:?}", e),
            Error::Sailfish(ref e) => error!("Sailfish: {e:?}"),
        }

        (self.status_code(), self.to_string()).into_response()
    }
}
