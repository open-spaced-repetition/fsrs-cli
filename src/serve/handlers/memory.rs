use axum::Json;

use super::{build_fsrs, reviews_to_item, to_err};
use crate::serve::models::*;

#[utoipa::path(
    post,
    path = "/memory/state",
    tag = "memory",
    request_body = MemoryStateRequest,
    responses(
        (status = 200, body = MemoryStateDto),
        (status = 400, body = ErrorResponse),
    )
)]
pub async fn memory_state(
    Json(req): Json<MemoryStateRequest>,
) -> Result<Json<MemoryStateDto>, Json<ErrorResponse>> {
    let fsrs = build_fsrs(&req.parameters)?;
    let item = reviews_to_item(&req.reviews);

    let starting_state = match (req.starting_stability, req.starting_difficulty) {
        (Some(s), Some(d)) => Some(MemoryStateDto { stability: s, difficulty: d }.into()),
        (None, None) => None,
        _ => {
            return Err(to_err(
                "Both starting_stability and starting_difficulty must be provided together",
            ));
        }
    };

    let state = fsrs.memory_state(item, starting_state).map_err(to_err)?;
    Ok(Json(MemoryStateDto::from(state)))
}

#[utoipa::path(
    post,
    path = "/memory/history",
    tag = "memory",
    request_body = MemoryStateRequest,
    responses(
        (status = 200, body = Vec<MemoryStateDto>),
        (status = 400, body = ErrorResponse),
    )
)]
pub async fn memory_history(
    Json(req): Json<MemoryStateRequest>,
) -> Result<Json<Vec<MemoryStateDto>>, Json<ErrorResponse>> {
    let fsrs = build_fsrs(&req.parameters)?;
    let item = reviews_to_item(&req.reviews);

    let starting_state = match (req.starting_stability, req.starting_difficulty) {
        (Some(s), Some(d)) => Some(MemoryStateDto { stability: s, difficulty: d }.into()),
        (None, None) => None,
        _ => {
            return Err(to_err(
                "Both starting_stability and starting_difficulty must be provided together",
            ));
        }
    };

    let states = fsrs
        .historical_memory_states(item, starting_state)
        .map_err(to_err)?;
    Ok(Json(states.into_iter().map(MemoryStateDto::from).collect()))
}

#[utoipa::path(
    post,
    path = "/memory/retrievability",
    tag = "memory",
    request_body = RetrievabilityRequest,
    responses(
        (status = 200, body = f32),
        (status = 400, body = ErrorResponse),
    )
)]
pub async fn retrievability(
    Json(req): Json<RetrievabilityRequest>,
) -> Result<Json<f32>, Json<ErrorResponse>> {
    let decay = req.decay.unwrap_or(fsrs::FSRS6_DEFAULT_DECAY);
    // Difficulty is not needed for retrievability calculation, so we can set it to any value (e.g. 0.0)
    let state: fsrs::MemoryState = MemoryStateDto { stability: req.stability, difficulty: 0.0 }.into();
    let r = fsrs::current_retrievability(state, req.ivl, decay);
    Ok(Json(r))
}

#[utoipa::path(
    post,
    path = "/memory/from-sm2",
    tag = "memory",
    request_body = FromSm2Request,
    responses(
        (status = 200, body = MemoryStateDto),
        (status = 400, body = ErrorResponse),
    )
)]
pub async fn from_sm2(
    Json(req): Json<FromSm2Request>,
) -> Result<Json<MemoryStateDto>, Json<ErrorResponse>> {
    let fsrs = build_fsrs(&req.parameters)?;
    let sm2_retention = req.sm2_retention.unwrap_or(0.9);
    let state = fsrs
        .memory_state_from_sm2(req.ease_factor, req.interval, sm2_retention)
        .map_err(to_err)?;
    Ok(Json(MemoryStateDto::from(state)))
}
