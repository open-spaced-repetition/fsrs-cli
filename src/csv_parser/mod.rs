mod revlog;

pub use revlog::{RevlogEntry, parse_revlog_csv};

use anyhow::Result;
use chrono::{DateTime, NaiveDate};
use chrono_tz::Tz;
use fsrs::{FSRSItem, FSRSReview};
use itertools::Itertools;
use std::path::Path;

/// Convert a timestamp (ms since epoch) to a date in the given timezone,
/// adjusting for `day_cutoff_hours` (e.g. 4 means 4:00 AM is the day boundary).
fn timestamp_to_date(timestamp_ms: i64, tz: &Tz, day_cutoff_hours: i64) -> Option<NaiveDate> {
    let dt = DateTime::from_timestamp_millis(timestamp_ms)?;
    let local_dt = dt.with_timezone(tz);
    let adjusted = local_dt - chrono::Duration::hours(day_cutoff_hours);
    Some(adjusted.date_naive())
}

/// Remove revlog entries before the last "first learn" block.
/// Keeps only entries from the last contiguous block of learning states (0=New, 1=Learning) onward.
fn remove_revlog_before_last_first_learn(entries: Vec<RevlogEntry>) -> Vec<RevlogEntry> {
    let is_learning_state = |entry: &RevlogEntry| matches!(entry.review_state.unwrap_or(0), 0 | 1);

    let mut last_learning_block_start = None;
    for i in (0..entries.len()).rev() {
        if is_learning_state(&entries[i]) {
            last_learning_block_start = Some(i);
        } else if last_learning_block_start.is_some() {
            break;
        }
    }

    if let Some(start) = last_learning_block_start {
        entries[start..].to_vec()
    } else {
        vec![]
    }
}

/// Convert a single card's entries into FSRSItems.
fn convert_card_entries(
    entries: Vec<RevlogEntry>,
    tz: &Tz,
    day_cutoff_hours: i64,
) -> Vec<(i64, FSRSItem)> {
    let entries = remove_revlog_before_last_first_learn(entries);

    if entries.len() < 2 {
        return vec![];
    }

    // Compute delta_t for each entry using timezone-aware dates
    let mut last_intervals: Vec<i32> = vec![0; entries.len()];
    if let Some(mut prev_date) = timestamp_to_date(entries[0].review_time, tz, day_cutoff_hours) {
        for (i, entry) in entries.iter().enumerate().skip(1) {
            if let Some(curr_date) = timestamp_to_date(entry.review_time, tz, day_cutoff_hours) {
                last_intervals[i] = (curr_date - prev_date).num_days() as i32;
                prev_date = curr_date;
            }
        }
    }

    // Create FSRSItems: one for each review from the second onward
    entries
        .iter()
        .enumerate()
        .skip(1)
        .filter_map(|(idx, entry)| {
            let reviews: Vec<FSRSReview> = entries
                .iter()
                .take(idx + 1)
                .enumerate()
                .map(|(i, r)| FSRSReview {
                    rating: r.review_rating as u32,
                    delta_t: last_intervals[i].max(0) as u32,
                })
                .collect();

            // Only include items where the current review has delta_t > 0
            if reviews.last().is_some_and(|r| r.delta_t > 0) {
                Some((entry.review_time, FSRSItem { reviews }))
            } else {
                None
            }
        })
        .collect()
}

/// Convert parsed revlog rows into FSRSItems.
///
/// 1. Group by card_id (rows must be pre-sorted by card_id, review_time)
/// 2. Remove entries before the last first-learn block per card
/// 3. Compute delta_t using timezone-aware day boundaries with day_cutoff
/// 4. Filter out items where current review delta_t == 0
/// 5. Sort all items by review_time across cards
pub fn revlog_to_fsrs_items(
    rows: &[RevlogEntry],
    tz: &Tz,
    day_cutoff_hours: i64,
) -> Vec<FSRSItem> {
    let mut items: Vec<(i64, FSRSItem)> = rows
        .iter()
        .cloned()
        .chunk_by(|r| r.card_id.clone())
        .into_iter()
        .flat_map(|(_, entries)| convert_card_entries(entries.collect(), tz, day_cutoff_hours))
        .collect();

    // Sort by review_time to maintain correct order across cards
    items.sort_by_cached_key(|(review_time, _)| *review_time);
    items.into_iter().map(|(_, item)| item).collect()
}

/// Load FSRSItems from a revlog CSV file.
///
/// `timezone` is an IANA timezone name (e.g. "Asia/Shanghai", "America/New_York").
/// `day_cutoff_hours` shifts the day start (e.g. 4 means 4:00 AM is the new day boundary).
pub fn load_fsrs_items_from_csv(
    path: &Path,
    timezone: &Tz,
    day_cutoff_hours: i64,
) -> Result<Vec<FSRSItem>> {
    let rows = parse_revlog_csv(path)?;
    Ok(revlog_to_fsrs_items(&rows, timezone, day_cutoff_hours))
}

/// Trait for CLI args that load review data from CSV.
pub trait CsvArgs {
    fn csv_path(&self) -> &Path;
    fn timezone_str(&self) -> &str;
    fn day_cutoff_hours(&self) -> i64;

    fn load_items(&self) -> Result<Vec<FSRSItem>> {
        let tz: Tz = self
            .timezone_str()
            .parse()
            .map_err(|_| anyhow::anyhow!("Invalid timezone: {}", self.timezone_str()))?;
        load_fsrs_items_from_csv(self.csv_path(), &tz, self.day_cutoff_hours())
    }
}
