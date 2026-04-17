use axum::Json;
use axum::extract::{FromRequest, Multipart, Request};
use axum::response::{IntoResponse, Response};
use fsrs::FSRS;

use super::{is_multipart, parse_csv_from_multipart, reviews_to_items};
use crate::serve::models::*;

/// Evaluate model fit against review data.
///
/// **JSON body:**
/// ```bash
/// curl -X POST http://localhost:7320/api/v1/evaluate \
///   -H "Content-Type: application/json" \
///   -d '{"items":[[{"rating":3,"delta_t":0},{"rating":3,"delta_t":1},{"rating":3,"delta_t":5}]]}'
/// ```
///
/// **CSV file upload:**
/// ```bash
/// curl -X POST http://localhost:7320/api/v1/evaluate \
///   -F "file=@revlog.csv" \
///   -F "timezone=Asia/Shanghai" \
///   -F "day_cutoff=4"
/// ```
#[utoipa::path(
    post,
    path = "/evaluate",
    tag = "evaluate",
    request_body = EvaluateRequest,
    responses(
        (status = 200, body = EvaluateResponse),
        (status = 400, body = ErrorResponse),
    )
)]
pub async fn evaluate_params(request: Request) -> Response {
    let headers = request.headers().clone();
    let (items, params) = if is_multipart(&headers) {
        let multipart = match Multipart::from_request(request, &()).await {
            Ok(m) => m,
            Err(e) => {
                return Json(ErrorResponse {
                    error: e.to_string(),
                })
                .into_response();
            }
        };
        match parse_csv_from_multipart(multipart).await {
            Ok(items) => (items, None),
            Err(e) => return Json(ErrorResponse { error: e }).into_response(),
        }
    } else {
        let body = match axum::body::to_bytes(request.into_body(), 50 * 1024 * 1024).await {
            Ok(b) => b,
            Err(e) => {
                return Json(ErrorResponse {
                    error: e.to_string(),
                })
                .into_response();
            }
        };
        match serde_json::from_slice::<EvaluateRequest>(&body) {
            Ok(req) => (reviews_to_items(&req.items), req.parameters),
            Err(e) => {
                return Json(ErrorResponse {
                    error: e.to_string(),
                })
                .into_response();
            }
        }
    };

    let params = params.unwrap_or_default();
    let fsrs = match FSRS::new(&params) {
        Ok(f) => f,
        Err(e) => {
            return Json(ErrorResponse {
                error: e.to_string(),
            })
            .into_response();
        }
    };

    match tokio::task::spawn_blocking(move || fsrs.evaluate(items, |_| true)).await {
        Ok(Ok(eval)) => Json(EvaluateResponse {
            log_loss: eval.log_loss,
            rmse_bins: eval.rmse_bins,
        })
        .into_response(),
        Ok(Err(e)) => Json(ErrorResponse {
            error: e.to_string(),
        })
        .into_response(),
        Err(e) => Json(ErrorResponse {
            error: e.to_string(),
        })
        .into_response(),
    }
}
