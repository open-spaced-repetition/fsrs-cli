use fsrs::{DEFAULT_PARAMETERS, MemoryState};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

// ============================================================
// Shared
// ============================================================

#[derive(Deserialize, Serialize, ToSchema)]
#[schema(example = json!({"stability": 2.3065, "difficulty": 2.118104}))]
pub struct MemoryStateDto {
    pub stability: f32,
    pub difficulty: f32,
}

impl From<MemoryState> for MemoryStateDto {
    fn from(s: MemoryState) -> Self {
        Self {
            stability: s.stability,
            difficulty: s.difficulty,
        }
    }
}

impl From<MemoryStateDto> for MemoryState {
    fn from(dto: MemoryStateDto) -> Self {
        Self {
            stability: dto.stability,
            difficulty: dto.difficulty,
        }
    }
}

#[derive(Deserialize, Serialize, ToSchema)]
#[schema(example = json!({"rating": 3, "delta_t": 5}))]
pub struct ReviewDto {
    pub rating: u32,
    pub delta_t: u32,
}

#[derive(Serialize, ToSchema)]
#[schema(example = json!({"error": "InvalidParameters"}))]
pub struct ErrorResponse {
    pub error: String,
}

// ============================================================
// schedule
// ============================================================

#[derive(Deserialize, ToSchema)]
#[schema(example = json!({
    "memory_state": {"stability": 2.3065, "difficulty": 2.118104},
    "retention": 0.9,
    "ivl": 1
}))]
pub struct NextStatesRequest {
    pub memory_state: Option<MemoryStateDto>,
    #[schema(default = 0.9)]
    pub retention: Option<f32>,
    #[schema(default = 0)]
    pub ivl: Option<u32>,
    pub parameters: Option<Vec<f32>>,
}

#[derive(Serialize, ToSchema)]
#[schema(example = json!({
    "again": {"memory_state": {"stability": 0.5713, "difficulty": 7.3945}, "interval": 0.5713},
    "hard":  {"memory_state": {"stability": 5.3188, "difficulty": 4.7529}, "interval": 5.3188},
    "good":  {"memory_state": {"stability": 7.3153, "difficulty": 2.1112}, "interval": 7.3153},
    "easy":  {"memory_state": {"stability": 11.6875, "difficulty": 1.0}, "interval": 11.6875}
}))]
pub struct NextStatesResponse {
    pub again: ItemStateDto,
    pub hard: ItemStateDto,
    pub good: ItemStateDto,
    pub easy: ItemStateDto,
}

#[derive(Serialize, ToSchema)]
#[schema(example = json!({"memory_state": {"stability": 7.3153, "difficulty": 2.1112}, "interval": 7.3153}))]
pub struct ItemStateDto {
    pub memory_state: MemoryStateDto,
    pub interval: f32,
}

// ============================================================
// memory state
// ============================================================

#[derive(Deserialize, ToSchema)]
#[schema(example = json!({
    "reviews": [
        {"rating": 3, "delta_t": 0},
        {"rating": 3, "delta_t": 1},
        {"rating": 3, "delta_t": 5},
        {"rating": 2, "delta_t": 10}
    ]
}))]
pub struct MemoryStateRequest {
    pub reviews: Vec<ReviewDto>,
    pub starting_stability: Option<f32>,
    pub starting_difficulty: Option<f32>,
    pub parameters: Option<Vec<f32>>,
}

// ============================================================
// memory retrievability
// ============================================================

#[derive(Deserialize, ToSchema)]
#[schema(example = json!({"stability": 2.3065, "ivl": 1.0}))]
pub struct RetrievabilityRequest {
    pub stability: f32,
    pub ivl: f32,
    pub decay: Option<f32>,
}

// ============================================================
// memory from-sm2
// ============================================================

#[derive(Deserialize, ToSchema)]
#[schema(example = json!({"ease_factor": 2.5, "interval": 30.0, "sm2_retention": 0.9}))]
pub struct FromSm2Request {
    pub ease_factor: f32,
    pub interval: f32,
    #[schema(default = 0.9)]
    pub sm2_retention: Option<f32>,
    pub parameters: Option<Vec<f32>>,
}

// ============================================================
// params
// ============================================================

fn params_response_example() -> serde_json::Value {
    serde_json::json!({
        "parameters": DEFAULT_PARAMETERS.to_vec(),
        "ratings": [
            {"value": 1, "label": "Again"},
            {"value": 2, "label": "Hard"},
            {"value": 3, "label": "Good"},
            {"value": 4, "label": "Easy"}
        ],
        "states": [
            {"value": 0, "label": "New"},
            {"value": 1, "label": "Learning"},
            {"value": 2, "label": "Review"},
            {"value": 3, "label": "Relearning"}
        ]
    })
}

#[derive(Serialize, ToSchema)]
#[schema(example = params_response_example)]
pub struct ParamsResponse {
    pub parameters: Vec<f32>,
    pub ratings: Vec<RatingInfo>,
    pub states: Vec<StateInfo>,
}

#[derive(Serialize, ToSchema)]
pub struct RatingInfo {
    pub value: u32,
    pub label: String,
}

#[derive(Serialize, ToSchema)]
pub struct StateInfo {
    pub value: u32,
    pub label: String,
}

// ============================================================
// optimize
// ============================================================

#[derive(Deserialize, ToSchema)]
#[schema(example = json!({
    "items": [
        [{"rating": 3, "delta_t": 0}, {"rating": 3, "delta_t": 1}, {"rating": 3, "delta_t": 5}],
        [{"rating": 3, "delta_t": 0}, {"rating": 2, "delta_t": 3}]
    ]
}))]
pub struct OptimizeRequest {
    pub items: Vec<Vec<ReviewDto>>,
    #[schema(default = true)]
    pub enable_short_term: Option<bool>,
    pub num_relearning_steps: Option<usize>,
}

// ============================================================
// evaluate
// ============================================================

#[derive(Deserialize, ToSchema)]
#[schema(example = json!({
    "items": [
        [{"rating": 3, "delta_t": 0}, {"rating": 3, "delta_t": 1}, {"rating": 3, "delta_t": 5}],
        [{"rating": 3, "delta_t": 0}, {"rating": 2, "delta_t": 3}]
    ]
}))]
pub struct EvaluateRequest {
    pub items: Vec<Vec<ReviewDto>>,
    pub parameters: Option<Vec<f32>>,
}

#[derive(Serialize, ToSchema)]
#[schema(example = json!({"log_loss": 0.0824, "rmse_bins": 0.0848}))]
pub struct EvaluateResponse {
    pub log_loss: f32,
    pub rmse_bins: f32,
}

// ============================================================
// evaluate-with-time-series-splits
// ============================================================

#[derive(Deserialize, ToSchema)]
#[schema(example = json!({
    "items": [
        [{"rating": 3, "delta_t": 0}, {"rating": 3, "delta_t": 1}],
        [{"rating": 3, "delta_t": 0}, {"rating": 3, "delta_t": 1}, {"rating": 3, "delta_t": 5}],
        [{"rating": 3, "delta_t": 0}, {"rating": 3, "delta_t": 1}, {"rating": 3, "delta_t": 5}, {"rating": 2, "delta_t": 10}],
        [{"rating": 3, "delta_t": 0}, {"rating": 3, "delta_t": 2}],
        [{"rating": 3, "delta_t": 0}, {"rating": 3, "delta_t": 2}, {"rating": 4, "delta_t": 5}],
        [{"rating": 1, "delta_t": 0}, {"rating": 3, "delta_t": 1}],
        [{"rating": 1, "delta_t": 0}, {"rating": 3, "delta_t": 1}, {"rating": 3, "delta_t": 2}]
    ]
}))]
pub struct EvaluateWithTimeSeriesSplitsRequest {
    pub items: Vec<Vec<ReviewDto>>,
    #[schema(default = true)]
    pub enable_short_term: Option<bool>,
    pub num_relearning_steps: Option<usize>,
}

// ============================================================
// simulate run
// ============================================================

#[derive(Deserialize, ToSchema)]
#[schema(example = json!({
    "deck_size": 100,
    "learn_span": 30,
    "retention": 0.9,
    "seed": 42
}))]
pub struct SimulateRunRequest {
    #[schema(default = 10000)]
    pub deck_size: Option<usize>,
    #[schema(default = 365)]
    pub learn_span: Option<usize>,
    #[schema(default = 1800.0)]
    pub max_cost_perday: Option<f32>,
    #[schema(default = 36500.0)]
    pub max_ivl: Option<f32>,
    #[schema(default = 0.9)]
    pub retention: Option<f32>,
    pub seed: Option<u64>,
    pub parameters: Option<Vec<f32>>,
    pub learn_limit: Option<usize>,
    pub review_limit: Option<usize>,
}

#[derive(Serialize, ToSchema)]
#[schema(example = json!({
    "total_reviews": 373,
    "total_learned": 100,
    "final_memorized": 94.9646,
    "avg_reviews_per_day": 12.4333,
    "avg_cost_per_day": 318.0167,
    "total_cost": 9540.501,
    "days": 30
}))]
pub struct SimulateRunResponse {
    pub total_reviews: usize,
    pub total_learned: usize,
    pub final_memorized: f32,
    pub avg_reviews_per_day: f32,
    pub avg_cost_per_day: f32,
    pub total_cost: f32,
    pub days: usize,
}

// ============================================================
// simulate optimal-retention
// ============================================================

#[derive(Deserialize, ToSchema)]
#[schema(example = json!({
    "deck_size": 10000,
    "learn_span": 365
}))]
pub struct OptimalRetentionRequest {
    #[schema(default = 10000)]
    pub deck_size: Option<usize>,
    #[schema(default = 365)]
    pub learn_span: Option<usize>,
    #[schema(default = 1800.0)]
    pub max_cost_perday: Option<f32>,
    #[schema(default = 36500.0)]
    pub max_ivl: Option<f32>,
    pub parameters: Option<Vec<f32>>,
    pub learn_limit: Option<usize>,
    pub review_limit: Option<usize>,
}

#[derive(Serialize, ToSchema)]
#[schema(example = json!({"optimal_retention": 0.8766}))]
pub struct OptimalRetentionResponse {
    pub optimal_retention: f32,
}

// ============================================================
// simulate workload
// ============================================================

#[derive(Deserialize, ToSchema)]
#[schema(example = json!({
    "retention": 0.9,
    "deck_size": 10000,
    "learn_span": 365
}))]
pub struct WorkloadRequest {
    #[schema(default = 0.9)]
    pub retention: Option<f32>,
    #[schema(default = 10000)]
    pub deck_size: Option<usize>,
    #[schema(default = 365)]
    pub learn_span: Option<usize>,
    #[schema(default = 1800.0)]
    pub max_cost_perday: Option<f32>,
    pub learn_limit: Option<usize>,
    pub review_limit: Option<usize>,
    pub parameters: Option<Vec<f32>>,
}

#[derive(Serialize, ToSchema)]
#[schema(example = json!({"expected_workload": 155.2765, "retention": 0.9}))]
pub struct WorkloadResponse {
    pub expected_workload: f32,
    pub retention: f32,
}

// ============================================================
// SSE progress
// ============================================================

#[derive(Serialize, ToSchema)]
#[schema(example = json!({"current": 50, "total": 100, "finished": false}))]
pub struct ProgressEvent {
    pub current: usize,
    pub total: usize,
    pub finished: bool,
}
