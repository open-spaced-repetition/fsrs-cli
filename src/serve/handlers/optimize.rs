use axum::Json;
use axum::extract::{FromRequest, Multipart, Request};
use axum::response::{IntoResponse, Response};
use fsrs::{CombinedProgressState, ComputeParametersInput, benchmark};
use std::sync::Arc;

use super::{is_multipart, parse_csv_from_multipart, reviews_to_items, wants_sse};
use crate::serve::models::*;
use crate::serve::sse;

/// Optimize FSRS parameters from review data.
///
/// Supports three request modes:
///
/// **1. JSON body:**
/// ```bash
/// curl -X POST http://localhost:7320/api/v1/optimize \
///   -H "Content-Type: application/json" \
///   -d '{"items":[[{"rating":3,"delta_t":0},{"rating":3,"delta_t":1},{"rating":3,"delta_t":5}]]}'
/// ```
///
/// **2. CSV file upload:**
/// ```bash
/// curl -X POST http://localhost:7320/api/v1/optimize \
///   -F "file=@revlog.csv" \
///   -F "timezone=Asia/Shanghai" \
///   -F "day_cutoff=4"
/// ```
///
/// **3. SSE stream** (add `Accept: text/event-stream` to mode 1 or 2):
/// ```bash
/// curl -X POST http://localhost:7320/api/v1/optimize \
///   -F "file=@revlog.csv" \
///   -H "Accept: text/event-stream"
/// ```
#[utoipa::path(
    post,
    path = "/optimize",
    tag = "optimize",
    request_body = OptimizeRequest,
    responses(
        (status = 200, body = Vec<f32>, description = "JSON array of 21 parameters, or SSE stream (Accept: text/event-stream)"),
        (status = 400, body = ErrorResponse),
    )
)]
pub async fn optimize_params(request: Request) -> Response {
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
        match serde_json::from_slice::<OptimizeRequest>(&body) {
            Ok(req) => (
                reviews_to_items(&req.items),
                req.enable_short_term.unwrap_or(true),
                req.num_relearning_steps,
            ),
            Err(e) => return Json(ErrorResponse { error: e.to_string() }).into_response(),
        }
    };

    if wants_sse(&headers) {
        let progress = CombinedProgressState::new_shared();
        let input = ComputeParametersInput {
            train_set: items,
            progress: Some(Arc::clone(&progress)),
            enable_short_term,
            num_relearning_steps,
        };
        return sse::progress_stream(progress, move || fsrs::compute_parameters(input))
            .into_response();
    }

    let input = ComputeParametersInput {
        train_set: items,
        progress: Some(CombinedProgressState::new_shared()),
        enable_short_term,
        num_relearning_steps,
    };

    match tokio::task::spawn_blocking(move || fsrs::compute_parameters(input)).await {
        Ok(Ok(parameters)) => Json(parameters).into_response(),
        Ok(Err(e)) => Json(ErrorResponse { error: e.to_string() }).into_response(),
        Err(e) => Json(ErrorResponse { error: e.to_string() }).into_response(),
    }
}

/// Fast parameter estimation (benchmark) from review data.
///
/// **JSON body:**
/// ```bash
/// curl -X POST http://localhost:7320/api/v1/benchmark \
///   -H "Content-Type: application/json" \
///   -d '{"items":[[{"rating":3,"delta_t":0},{"rating":3,"delta_t":1},{"rating":3,"delta_t":5}]]}'
/// ```
///
/// **CSV file upload:**
/// ```bash
/// curl -X POST http://localhost:7320/api/v1/benchmark \
///   -F "file=@revlog.csv" \
///   -F "timezone=Asia/Shanghai" \
///   -F "day_cutoff=4"
/// ```
#[utoipa::path(
    post,
    path = "/benchmark",
    tag = "optimize",
    request_body = OptimizeRequest,
    responses(
        (status = 200, body = Vec<f32>, description = "21 optimized parameters"),
        (status = 400, body = ErrorResponse),
    )
)]
pub async fn benchmark_params(request: Request) -> Response {
    let headers = request.headers().clone();
    let items = if is_multipart(&headers) {
        let multipart = match Multipart::from_request(request, &()).await {
            Ok(m) => m,
            Err(e) => return Json(ErrorResponse { error: e.to_string() }).into_response(),
        };
        match parse_csv_from_multipart(multipart).await {
            Ok(items) => items,
            Err(e) => return Json(ErrorResponse { error: e }).into_response(),
        }
    } else {
        let body = match axum::body::to_bytes(request.into_body(), 50 * 1024 * 1024).await {
            Ok(b) => b,
            Err(e) => return Json(ErrorResponse { error: e.to_string() }).into_response(),
        };
        match serde_json::from_slice::<OptimizeRequest>(&body) {
            Ok(req) => reviews_to_items(&req.items),
            Err(e) => return Json(ErrorResponse { error: e.to_string() }).into_response(),
        }
    };

    let input = ComputeParametersInput {
        train_set: items,
        progress: Some(CombinedProgressState::new_shared()),
        enable_short_term: true,
        num_relearning_steps: None,
    };

    match tokio::task::spawn_blocking(move || benchmark(input)).await {
        Ok(parameters) => Json(parameters).into_response(),
        Err(e) => Json(ErrorResponse { error: e.to_string() }).into_response(),
    }
}
