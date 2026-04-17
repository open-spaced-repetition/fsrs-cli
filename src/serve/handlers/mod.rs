mod evaluate;
mod evaluate_with_time_series_splits;
mod memory;
mod optimize;
mod params;
mod schedule;
mod simulate;

use axum::Json;
use axum::http::HeaderMap;
use fsrs::{FSRSItem, FSRSReview};
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use super::models::*;

pub fn routes() -> OpenApiRouter {
    OpenApiRouter::new()
        // schedule
        .routes(routes!(schedule::next_states))
        // memory
        .routes(routes!(memory::memory_state))
        .routes(routes!(memory::memory_history))
        .routes(routes!(memory::retrievability))
        .routes(routes!(memory::from_sm2))
        // params
        .routes(routes!(params::get_default_params))
        // optimize & benchmark
        .routes(routes!(optimize::optimize_params))
        .routes(routes!(optimize::benchmark_params))
        // evaluate
        .routes(routes!(evaluate::evaluate_params))
        .routes(routes!(evaluate_with_time_series_splits::evaluate_with_time_series_splits))
        // simulate
        .routes(routes!(simulate::simulate_run))
        .routes(routes!(simulate::optimal_retention))
        .routes(routes!(simulate::workload))
}

fn to_err(e: impl std::fmt::Display) -> Json<ErrorResponse> {
    Json(ErrorResponse {
        error: e.to_string(),
    })
}

fn build_fsrs(params: &Option<Vec<f32>>) -> Result<fsrs::FSRS, Json<ErrorResponse>> {
    let p = params.as_deref().unwrap_or(&[]);
    fsrs::FSRS::new(p).map_err(to_err)
}

fn reviews_to_item(reviews: &[ReviewDto]) -> FSRSItem {
    FSRSItem {
        reviews: reviews
            .iter()
            .map(|r| FSRSReview {
                rating: r.rating,
                delta_t: r.delta_t,
            })
            .collect(),
    }
}

fn reviews_to_items(items: &[Vec<ReviewDto>]) -> Vec<FSRSItem> {
    items.iter().map(|reviews| reviews_to_item(reviews)).collect()
}

fn is_multipart(headers: &HeaderMap) -> bool {
    headers
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.contains("multipart/form-data"))
}

fn wants_sse(headers: &HeaderMap) -> bool {
    headers
        .get("accept")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.contains("text/event-stream"))
}

async fn parse_csv_from_multipart(
    mut multipart: axum::extract::Multipart,
) -> Result<Vec<FSRSItem>, String> {
    let mut csv_data: Option<Vec<u8>> = None;
    let mut timezone: String = "UTC".to_string();
    let mut day_cutoff: i64 = 0;

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().unwrap_or("").to_string();
        match name.as_str() {
            "file" => {
                csv_data = Some(field.bytes().await.map_err(|e| e.to_string())?.to_vec());
            }
            "timezone" => {
                timezone = field.text().await.map_err(|e| e.to_string())?;
            }
            "day_cutoff" => {
                let val = field.text().await.map_err(|e| e.to_string())?;
                day_cutoff = val.parse().unwrap_or(0);
            }
            _ => {}
        }
    }

    let tz: chrono_tz::Tz = timezone
        .parse()
        .map_err(|_| format!("Invalid timezone: {timezone}"))?;
    let data = csv_data.ok_or("Missing 'file' field in multipart form")?;

    let tmp = tempfile::NamedTempFile::new().map_err(|e| e.to_string())?;
    std::fs::write(tmp.path(), &data).map_err(|e| e.to_string())?;

    crate::csv_parser::load_fsrs_items_from_csv(tmp.path(), &tz, day_cutoff)
        .map_err(|e| e.to_string())
}
