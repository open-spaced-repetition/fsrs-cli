use axum::Json;
use fsrs::{SimulatorConfig, expected_workload, simulate};

use super::to_err;
use crate::serve::models::*;

fn build_config(
    deck_size: usize,
    learn_span: usize,
    max_cost_perday: f32,
    max_ivl: f32,
    learn_limit: Option<usize>,
    review_limit: Option<usize>,
) -> SimulatorConfig {
    SimulatorConfig {
        deck_size,
        learn_span,
        max_cost_perday,
        max_ivl,
        learn_limit: learn_limit.unwrap_or(usize::MAX),
        review_limit: review_limit.unwrap_or(usize::MAX),
        ..Default::default()
    }
}

#[utoipa::path(
    post,
    path = "/simulate/run",
    tag = "simulate",
    request_body = SimulateRunRequest,
    responses(
        (status = 200, body = SimulateRunResponse),
        (status = 400, body = ErrorResponse),
    )
)]
pub async fn simulate_run(
    Json(req): Json<SimulateRunRequest>,
) -> Result<Json<SimulateRunResponse>, Json<ErrorResponse>> {
    let params = req.parameters.unwrap_or_default();
    let retention = req.retention.unwrap_or(0.9);
    let config = build_config(
        req.deck_size.unwrap_or(10000),
        req.learn_span.unwrap_or(365),
        req.max_cost_perday.unwrap_or(1800.0),
        req.max_ivl.unwrap_or(36500.0),
        req.learn_limit,
        req.review_limit,
    );

    let result = simulate(&config, &params, retention, req.seed, None).map_err(to_err)?;

    let total_reviews: usize = result.review_cnt_per_day.iter().sum();
    let total_learned: usize = result.learn_cnt_per_day.iter().sum();
    let total_cost: f32 = result.cost_per_day.iter().sum();
    let days = result.review_cnt_per_day.len();
    let final_memorized = *result.memorized_cnt_per_day.last().unwrap_or(&0.0);

    Ok(Json(SimulateRunResponse {
        total_reviews,
        total_learned,
        final_memorized,
        avg_reviews_per_day: if days > 0 {
            total_reviews as f32 / days as f32
        } else {
            0.0
        },
        avg_cost_per_day: if days > 0 {
            total_cost / days as f32
        } else {
            0.0
        },
        total_cost,
        days,
    }))
}

#[utoipa::path(
    post,
    path = "/simulate/optimal-retention",
    tag = "simulate",
    request_body = OptimalRetentionRequest,
    responses(
        (status = 200, body = OptimalRetentionResponse),
        (status = 400, body = ErrorResponse),
    )
)]
pub async fn optimal_retention(
    Json(req): Json<OptimalRetentionRequest>,
) -> Result<Json<OptimalRetentionResponse>, Json<ErrorResponse>> {
    let params = req.parameters.unwrap_or_default();
    let config = build_config(
        req.deck_size.unwrap_or(10000),
        req.learn_span.unwrap_or(365),
        req.max_cost_perday.unwrap_or(1800.0),
        req.max_ivl.unwrap_or(36500.0),
        req.learn_limit,
        req.review_limit,
    );

    let retention =
        fsrs::optimal_retention(&config, &params, |_| true, None, None).map_err(to_err)?;

    Ok(Json(OptimalRetentionResponse {
        optimal_retention: retention,
    }))
}

#[utoipa::path(
    post,
    path = "/simulate/workload",
    tag = "simulate",
    request_body = WorkloadRequest,
    responses(
        (status = 200, body = WorkloadResponse),
        (status = 400, body = ErrorResponse),
    )
)]
pub async fn workload(
    Json(req): Json<WorkloadRequest>,
) -> Result<Json<WorkloadResponse>, Json<ErrorResponse>> {
    let params = req.parameters.unwrap_or_default();
    let retention = req.retention.unwrap_or(0.9);
    let config = build_config(
        req.deck_size.unwrap_or(10000),
        req.learn_span.unwrap_or(365),
        req.max_cost_perday.unwrap_or(1800.0),
        36500.0,
        req.learn_limit,
        req.review_limit,
    );

    let wl = expected_workload(&params, retention, &config).map_err(to_err)?;

    Ok(Json(WorkloadResponse {
        expected_workload: wl,
        retention,
    }))
}
