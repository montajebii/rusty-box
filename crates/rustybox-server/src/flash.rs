//! One-shot flash messages. Without JavaScript there is no toast queue like
//! v1's, so actions redirect with a short code (`?m=…`) that the next page
//! renders as a fading toast. Codes are resolved server-side to fixed texts,
//! so user-controlled strings never land in markup unescaped.

/// Resolve a flash code (and optional payload: names/counts) to the message.
pub fn resolve(code: &str, payload: Option<&str>) -> Option<String> {
    let msg = match code {
        "dropped" => "💧 Dropped into the inbox — file it later".to_string(),
        "filed" => {
            let names = payload.unwrap_or_default();
            if names.is_empty() {
                return None;
            }
            format!("💧 Filed into {names}")
        }
        "empty" => "⚠️ An idea can’t be empty".to_string(),
        "too_long" => "⚠️ That idea is too long".to_string(),
        "updated" => "✏️ Idea updated".to_string(),
        "buckets_updated" => "🗂 Buckets updated".to_string(),
        "deleted" => "🗑 Idea removed".to_string(),
        "not_found" => "⚠️ That idea isn’t here anymore".to_string(),
        "bucket_deleted" => {
            format!("🗂 Bucket removed from {} idea(s)", payload.unwrap_or("0"))
        }
        "empty_bucket" => "🪣 The bucket is empty — drop an idea in first".to_string(),
        "dipped" => "🥄 Dipped up an idea".to_string(),
        "imported" => {
            // payload is "<imported>,<skipped>"
            let (imported, skipped) = payload
                .unwrap_or("0,0")
                .split_once(',')
                .unwrap_or(("0", "0"));
            format!("⬇ Imported {imported} idea(s), skipped {skipped}")
        }
        "import_failed" => "⚠️ That doesn’t look like a JSON export".to_string(),
        "save_failed" => {
            "⚠️ Could not write the data file — the idea lives in memory only".to_string()
        }
        _ => return None,
    };
    Some(msg)
}

/// Build the query suffix for a redirect: `?m=<code>` (+ `&p=<payload>`).
pub fn redirect_query(code: &str, payload: Option<&str>) -> String {
    match payload {
        Some(p) => format!("?m={}&p={}", code, crate::view::url_encode(p)),
        None => format!("?m={code}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_codes_resolve() {
        assert_eq!(
            resolve("dropped", None).unwrap(),
            "💧 Dropped into the inbox — file it later"
        );
        assert_eq!(
            resolve("filed", Some("Work + Side")).unwrap(),
            "💧 Filed into Work + Side"
        );
    }

    #[test]
    fn filed_without_names_is_dropped_silently() {
        assert!(resolve("filed", None).is_none());
        assert!(resolve("filed", Some("")).is_none());
    }

    #[test]
    fn unknown_codes_resolve_to_nothing() {
        assert!(resolve("nope", None).is_none());
        assert!(resolve("", None).is_none());
    }

    #[test]
    fn import_message_shows_both_counts() {
        assert_eq!(
            resolve("imported", Some("3,1")).unwrap(),
            "⬇ Imported 3 idea(s), skipped 1"
        );
        assert_eq!(
            resolve("imported", None).unwrap(),
            "⬇ Imported 0 idea(s), skipped 0"
        );
    }

    #[test]
    fn redirect_queries_are_encoded() {
        assert_eq!(redirect_query("dropped", None), "?m=dropped");
        assert_eq!(
            redirect_query("filed", Some("side projects")),
            "?m=filed&p=side%20projects"
        );
    }
}
