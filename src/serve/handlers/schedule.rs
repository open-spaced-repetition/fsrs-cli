use axum::Json;

use super::{build_fsrs, to_err};
use crate::serve::models::*;

#[utoipa::path(
    post,
    path = "/schedule/next-states",
    tag = "schedule",
    request_body = NextStatesRequest,
    responses(
        (status = 200, body = NextStatesResponse),
        (status = 400, body = ErrorResponse),
    )
)]
pub async fn next_states(
    Json(req): Json<NextStatesRequest>,
) -> Result<Json<NextStatesResponse>, Json<ErrorResponse>> {
    let fsrs = build_fsrs(&req.parameters)?;
    let retention = req.retention.unwrap_or(0.9);
    let ivl = req.ivl.unwrap_or(0);

    let current_state = req.memory_state.map(fsrs::MemoryState::from);

    let states = fsrs
        .next_states(current_state, retention, ivl)
        .map_err(to_err)?;

    let to_dto = |s: fsrs::ItemState| ItemStateDto {
        memory_state: s.memory.into(),
        interval: s.interval,
    };

    Ok(Json(NextStatesResponse {
        again: to_dto(states.again),
        hard: to_dto(states.hard),
        good: to_dto(states.good),
        easy: to_dto(states.easy),
    }))
}
