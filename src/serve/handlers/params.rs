use axum::Json;
use fsrs::DEFAULT_PARAMETERS;

use crate::serve::models::*;

#[utoipa::path(
    get,
    path = "/params/default",
    tag = "params",
    responses(
        (status = 200, body = ParamsResponse),
    )
)]
pub async fn get_default_params() -> Json<ParamsResponse> {
    Json(ParamsResponse {
        parameters: DEFAULT_PARAMETERS.to_vec(),
        ratings: vec![
            RatingInfo { value: 1, label: "Again".to_string() },
            RatingInfo { value: 2, label: "Hard".to_string() },
            RatingInfo { value: 3, label: "Good".to_string() },
            RatingInfo { value: 4, label: "Easy".to_string() },
        ],
        states: vec![
            StateInfo { value: 0, label: "New".to_string() },
            StateInfo { value: 1, label: "Learning".to_string() },
            StateInfo { value: 2, label: "Review".to_string() },
            StateInfo { value: 3, label: "Relearning".to_string() },
        ],
    })
}
