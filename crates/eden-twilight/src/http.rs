use error_stack::Report;
use serde::de::DeserializeOwned;
use thiserror::Error;
use twilight_http::{
    Response,
    api_error::ApiError,
    response::{DeserializeBodyError, ResponseFuture, StatusCode},
};

// Retrieved from: https://discord.com/developers/docs/topics/opcodes-and-status-codes#http
const MISSING_PERMISSIONS_CODE: u64 = 50013;

#[derive(Debug, Error)]
#[error("Failed to perform HTTP request to Discord")]
pub enum HttpRequestError {
    /// Errors related to the HTTP request itself (network, status codes, etc).
    #[error(transparent)]
    General(#[from] twilight_http::Error),

    /// Errors related to parsing the response body.
    Deserialize(#[from] DeserializeBodyError),
}

impl HttpRequestError {
    /// Deduce the high-level reason for the error.
    #[must_use]
    pub fn reason(&self) -> HttpFailReason {
        use twilight_http::error::ErrorType as TwilightErrorType;

        let kind = match self {
            Self::Deserialize(..) => return HttpFailReason::Deserialize,
            Self::General(err) => err.kind(),
        };

        // TOO_MANY_REQUESTS is handled internally by twilight
        match kind {
            TwilightErrorType::Json | TwilightErrorType::Parsing { .. } => {
                HttpFailReason::Deserialize
            }
            TwilightErrorType::RequestTimedOut => HttpFailReason::TimedOut,
            TwilightErrorType::Unauthorized
            | TwilightErrorType::Response {
                status: StatusCode::UNAUTHORIZED,
                ..
            } => HttpFailReason::Unauthorized,
            TwilightErrorType::Response {
                error: ApiError::General(error),
                ..
            } if error.code == MISSING_PERMISSIONS_CODE => HttpFailReason::MissingPermissions,
            TwilightErrorType::RequestError => HttpFailReason::Connection,
            _ => HttpFailReason::Unknown,
        }
    }
}

/// Categorizes the underlying cause of the API error for easier handling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HttpFailReason {
    Connection,
    Deserialize,
    MissingPermissions,
    TimedOut,
    Unauthorized,
    ServiceUnavailable,
    Unknown,
}

#[allow(async_fn_in_trait)]
pub trait ResponseFutureExt {
    type Output;

    async fn perform(self) -> Result<Response<Self::Output>, Report<HttpRequestError>>;
    async fn model(self) -> Result<Self::Output, Report<HttpRequestError>>
    where
        Self::Output: DeserializeOwned;
}

impl<R, T> ResponseFutureExt for T
where
    R: Send + Unpin,
    T: IntoFuture<Output = HttpResult<R>, IntoFuture = ResponseFuture<R>>,
{
    type Output = R;

    async fn perform(self) -> Result<Response<Self::Output>, Report<HttpRequestError>> {
        self.await.simplify_error()
    }

    async fn model(self) -> Result<Self::Output, Report<HttpRequestError>>
    where
        Self::Output: DeserializeOwned,
    {
        self.await.simplify_error()?.model().await.simplify_error()
    }
}

pub trait HttpResultExt {
    type Ok;

    fn simplify_error(self) -> Result<Self::Ok, Report<HttpRequestError>>;
}

type HttpResult<T> = Result<Response<T>, twilight_http::Error>;

impl<T> HttpResultExt for HttpResult<T>
where
    T: Send + Unpin,
{
    type Ok = Response<T>;

    fn simplify_error(self) -> Result<Response<T>, Report<HttpRequestError>> {
        match self {
            Ok(okay) => Ok(okay),
            Err(error) => Err(Report::new(HttpRequestError::General(error))),
        }
    }
}

impl<T> HttpResultExt for Result<T, DeserializeBodyError>
where
    T: Send + Unpin + DeserializeOwned,
{
    type Ok = T;

    fn simplify_error(self) -> Result<T, Report<HttpRequestError>> {
        match self {
            Ok(okay) => Ok(okay),
            Err(error) => Err(Report::new(HttpRequestError::Deserialize(error))),
        }
    }
}
