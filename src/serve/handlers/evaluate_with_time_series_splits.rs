use axum::Json;
use axum::extract::{FromRequest, Multipart, Request};
use axum::response::{IntoResponse, Response};
use fsrs::ComputeParametersInput;
use std::sync::{Arc, Mutex};

use super::{is_multipart, parse_csv_from_multipart, reviews_to_items, wants_sse};
use crate::serve::models::*;
use crate::serve::sse;

/// Evaluate with time-series cross-validation (train/test splits).
///
/// **JSON body:**
/// ```bash
/// curl -X POST http://localhost:7320/api/v1/evaluate-with-time-series-splits \
///   -H "Content-Type: application/json" \
///   -d '{"items":[[{"rating":3,"delta_t":0},{"rating":3,"delta_t":1}], ...]}'
/// ```
///
/// **CSV file upload:**
/// ```bash
/// curl -X POST http://localhost:7320/api/v1/evaluate-with-time-series-splits \
///   -F "file=@revlog.csv" \
///   -F "timezone=Asia/Shanghai" \
///   -F "day_cutoff=4"
/// ```
///
/// **SSE stream** (add `Accept: text/event-stream` to mode 1 or 2):
/// ```bash
/// curl -X POST http://localhost:7320/api/v1/evaluate-with-time-series-splits \
///   -F "file=@revlog.csv" \
///   -H "Accept: text/event-stream"
/// ```
#[utoipa::path(
    post,
    path = "/evaluate-with-time-series-splits",
    tag = "evaluate",
    request_body = EvaluateWithTimeSeriesSplitsRequest,
    responses(
        (status = 200, body = EvaluateResponse, description = "JSON response or SSE stream (Accept: text/event-stream)"),
        (status = 400, body = ErrorResponse),
    )
)]
pub async fn evaluate_with_time_series_splits(request: Request) -> Response {
    let headers = request.headers().clone();
    let (items, enable_short_term, num_relearning_steps) = if is_multipart(&headers) {
        let multipart = match Multipart::from_request(request, &()).await {
            Ok(m) => m,
            Err(e) => return Json(ErrorResponse { error: e.to_string() }).into_response(),
        };
        match parse_csv_from_multipart(multipart).await {
            Ok(items) => (items, true, None),
            Err(e) => return Json(ErrorResponse { error: e }).into_response(),
        }
    } else {
        let body = match axum::body::to_bytes(request.into_body(), 50 * 1024 * 1024).await {
            Ok(b) => b,
            Err(e) => return Json(ErrorResponse { error: e.to_string() }).into_response(),
        };
        match serde_json::from_slice::<EvaluateWithTimeSeriesSplitsRequest>(&body) {
            Ok(req) => (
                reviews_to_items(&req.items),
                req.enable_short_term.unwrap_or(true),
                req.num_relearning_steps,
            ),
            Err(e) => return Json(ErrorResponse { error: e.to_string() }).into_response(),
        }
    };

    let input = ComputeParametersInput {
        train_set: items,
        progress: None,
        enable_short_term,
        num_relearning_steps,
    };

    if wants_sse(&headers) {
        let progress = Arc::new(Mutex::new(sse::SimpleProgress {
            current: 0,
            total: 0,
        }));
        let progress_for_cb = Arc::clone(&progress);
        return sse::progress_stream(progress, move || {
            let eval = fsrs::evaluate_with_time_series_splits(input, |p| {
                let mut state = progress_for_cb.lock().unwrap();
                state.current = p.current;
                state.total = p.total;
                true
            })?;
            Ok::<_, fsrs::FSRSError>(EvaluateResponse {
                log_loss: eval.log_loss,
                rmse_bins: eval.rmse_bins,
            })
        })
        .into_response();
    }

    match tokio::task::spawn_blocking(move || {
        fsrs::evaluate_with_time_series_splits(input, |_| true)
    })
    .await
    {
        Ok(Ok(eval)) => Json(EvaluateResponse {
            log_loss: eval.log_loss,
            rmse_bins: eval.rmse_bins,
        })
        .into_response(),
        Ok(Err(e)) => Json(ErrorResponse { error: e.to_string() }).into_response(),
        Err(e) => Json(ErrorResponse { error: e.to_string() }).into_response(),
    }
}
