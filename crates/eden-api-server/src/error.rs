// use axum::http::StatusCode;
// use axum::response::{IntoResponse, Response};
// use eden_sqlite::error::{ReportExt, SqlErrorType};
// use error_stack::Report;
// use std::any::{Any, TypeId};
// use std::error::Error;

// use eden_api_types::common::{ApiError, ApiErrorType};

// pub type ApiResult<T> = std::result::Result<T, Box<dyn MaybeApiError>>;

// pub fn not_found() -> Box<dyn MaybeApiError> {
//     Box::new(ApiError {
//         error: ApiErrorType::NotFound,
//         message: "Cannot find resource".into(),
//     })
// }

// pub trait MaybeApiError: Send + 'static {
//     fn response(&self) -> Response;

//     fn get_type_id(&self) -> TypeId {
//         TypeId::of::<Self>()
//     }
// }

// impl dyn MaybeApiError {
//     pub fn is<T: Any>(&self) -> bool {
//         self.get_type_id() == TypeId::of::<T>()
//     }
// }

// impl MaybeApiError for ApiError {
//     fn response(&self) -> Response {
//         let code = match self.error {
//             ApiErrorType::Internal => StatusCode::INTERNAL_SERVER_ERROR,
//             ApiErrorType::NotFound => StatusCode::NOT_FOUND,
//             ApiErrorType::Request => StatusCode::BAD_REQUEST,
//         };

//         let mut response = axum::Json(self).into_response();
//         *response.status_mut() = code;
//         response
//     }
// }
