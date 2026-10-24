//! Form extraction backed by `serde_html_form`.
//!
//! axum's built-in `Form` uses `serde_urlencoded`, which cannot deserialize
//! `Vec<String>` fields — the integration suite caught filing a single
//! bucket (`buckets=Work`) being rejected with 422. `serde_html_form`
//! handles both `buckets=Work` and `buckets=Work&buckets=Side`.

use axum::extract::FromRequest;
use axum::http::StatusCode;
use serde::de::DeserializeOwned;

pub struct HtmlForm<T>(pub T);

impl<S, T> FromRequest<S> for HtmlForm<T>
where
    S: Send + Sync,
    T: DeserializeOwned,
{
    type Rejection = (StatusCode, String);

    async fn from_request(req: axum::extract::Request, state: &S) -> Result<Self, Self::Rejection> {
        let bytes = axum::body::Bytes::from_request(req, state)
            .await
            .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
        serde_html_form::from_bytes::<T>(&bytes)
            .map(HtmlForm)
            .map_err(|e| (StatusCode::UNPROCESSABLE_ENTITY, e.to_string()))
    }
}
