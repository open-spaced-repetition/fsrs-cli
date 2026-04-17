use serde::Serialize;

/// Print a serializable value as JSON or with a custom human display closure.
pub fn print_with<T: Serialize, F: FnOnce(&T)>(
    value: &T,
    json: bool,
    display: F,
) -> anyhow::Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(value)?);
    } else {
        display(value);
    }
    Ok(())
}
