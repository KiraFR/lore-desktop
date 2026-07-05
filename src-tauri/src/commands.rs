use crate::lore::{events_with_tag, run_lore, LoreEvent};

#[tauri::command]
pub fn ping() -> String {
    "pong".to_string()
}

/// True iff any stored identity has an `expires` in the future.
fn is_authenticated_from(events: &[crate::lore::LoreEvent], now_ms: i64) -> bool {
    events_with_tag(events, "authIdentity").iter().any(|d| {
        d.get("expires").and_then(|e| e.as_i64()).map(|exp| exp > now_ms).unwrap_or(false)
    })
}

#[tauri::command]
pub fn lore_is_authenticated() -> Result<bool, String> {
    let events = run_lore(&["auth", "list"])?;
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    Ok(is_authenticated_from(&events, now_ms))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lore::parse_events;

    #[test]
    fn detects_valid_identity() {
        let events = parse_events(include_str!("../tests/fixtures/auth_list.ndjson")).unwrap();
        // The captured identity's `expires` is in the past relative to a far-future clock,
        // and in the future relative to a zero clock.
        assert!(is_authenticated_from(&events, 0));
    }
}
