use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::path::Path;

/// A row from a ts-fsrs style revlog CSV export.
///
/// Expected columns: card_id, review_time, review_rating, review_state, review_duration
///
/// - `card_id`: Card identifier (string or integer)
/// - `review_time`: Timestamp in milliseconds since epoch
/// - `review_rating`: Rating 1-4 (Again, Hard, Good, Easy)
/// - `review_state`: 0=New, 1=Learning, 2=Review, 3=Relearning
/// - `review_duration`: Time spent on review in milliseconds
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct RevlogEntry {
    pub card_id: String,
    pub review_time: i64,
    pub review_rating: u8,
    pub review_state: Option<u8>,
    pub review_duration: Option<u32>,
}

/// Parse a revlog-format CSV file (ts-fsrs format).
pub fn parse_revlog_csv(path: &Path) -> Result<Vec<RevlogEntry>> {
    if !path.exists() {
        bail!("CSV file not found: {}", path.display());
    }
    if !path.is_file() {
        bail!("Path is not a file: {}", path.display());
    }

    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_path(path)
        .context("Failed to open CSV file")?;

    let mut rows: Vec<RevlogEntry> = rdr
        .deserialize::<RevlogEntry>()
        .map(|r| r.context("Failed to parse revlog CSV row"))
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .filter(|row| (1..=4).contains(&row.review_rating))
        .collect();

    rows.sort_by_cached_key(|r| (r.card_id.clone(), r.review_time));

    Ok(rows)
}
