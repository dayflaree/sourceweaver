pub const SUPPORTED_SINGLE_ID_REFERENCE_KEYS: &[&str] = &[
    "parentid",
    "groupid",
    "visgroupid",
    "sideid",
    "solidid",
    "entityid",
    "nodeid",
];

pub const SUPPORTED_LIST_ID_REFERENCE_KEYS: &[&str] = &["sides"];

pub const KNOWN_ID_LIKE_NON_REFERENCE_KEYS: &[&str] = &[
    "id",
    "hammerid",
    "hammeruniqueid",
    "visgroupshown",
    "visgroupautoshown",
];

pub fn is_single_id_reference_key(key: &str) -> bool {
    SUPPORTED_SINGLE_ID_REFERENCE_KEYS.contains(&key)
}

pub fn is_list_id_reference_key(key: &str) -> bool {
    SUPPORTED_LIST_ID_REFERENCE_KEYS.contains(&key)
}

pub fn is_supported_id_reference_key(key: &str) -> bool {
    is_single_id_reference_key(key) || is_list_id_reference_key(key)
}

pub fn is_known_id_like_non_reference_key(key: &str) -> bool {
    let normalized = key.to_ascii_lowercase();
    KNOWN_ID_LIKE_NON_REFERENCE_KEYS.contains(&normalized.as_str())
}

pub fn is_suspected_id_reference_key(key: &str, value: &str) -> bool {
    if is_supported_id_reference_key(key) || is_known_id_like_non_reference_key(key) {
        return false;
    }

    let normalized = key.to_ascii_lowercase();
    if normalized.ends_with("grid") {
        return false;
    }
    let id_like_name = normalized.ends_with("id") || normalized.ends_with("ids");
    id_like_name && is_numeric_id_or_id_list(value)
}

pub fn is_numeric_id_or_id_list(value: &str) -> bool {
    let mut saw_part = false;
    for part in value.split_whitespace() {
        saw_part = true;
        if part.parse::<i64>().is_err() {
            return false;
        }
    }
    saw_part
}

pub fn supported_id_reference_summary() -> String {
    format!(
        "single: {}; list: {}",
        SUPPORTED_SINGLE_ID_REFERENCE_KEYS.join(", "),
        SUPPORTED_LIST_ID_REFERENCE_KEYS.join(", ")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_supported_and_suspected_keys() {
        assert!(is_single_id_reference_key("groupid"));
        assert!(is_list_id_reference_key("sides"));
        assert!(!is_suspected_id_reference_key("groupid", "10"));
        assert!(!is_suspected_id_reference_key("hammerid", "10"));
        assert!(!is_suspected_id_reference_key("bSnapToGrid", "1"));
        assert!(is_suspected_id_reference_key("targetid", "10"));
        assert!(is_suspected_id_reference_key("nodeids", "10 11 12"));
        assert!(!is_suspected_id_reference_key("nodeids", "10 bogus"));
    }
}
