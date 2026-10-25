//! One-shot flash messages. Without JavaScript there is no toast queue like
//! v1's, so actions redirect with a short code (`?m=…`) that the next page
//! renders as a fading toast. Codes are resolved server-side to fixed texts
//! with a fixed *kind* (`ok` / `warn` / `err`), so user-controlled strings
//! never land in markup unescaped and the toast's icon is drawn by CSS from
//! the kind alone.

/// How a toast reads, rendered as the CSS modifier class on `.toast`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlashKind {
    Ok,
    Warn,
    Err,
}

impl FlashKind {
    pub fn class(self) -> &'static str {
        match self {
            FlashKind::Ok => "toast-ok",
            FlashKind::Warn => "toast-warn",
            FlashKind::Err => "toast-err",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Flash {
    pub kind: FlashKind,
    pub text: String,
}

impl Flash {
    fn ok(text: impl Into<String>) -> Flash {
        Flash {
            kind: FlashKind::Ok,
            text: text.into(),
        }
    }

    fn warn(text: impl Into<String>) -> Flash {
        Flash {
            kind: FlashKind::Warn,
            text: text.into(),
        }
    }
}

/// Resolve a flash code (and optional payload: names/counts) to the message.
pub fn resolve(code: &str, payload: Option<&str>) -> Option<Flash> {
    let flash = match code {
        "dropped" => Flash::ok("Dropped into the box — file it later if you like"),
        "filed" => {
            let names = payload.unwrap_or_default();
            if names.is_empty() {
                return None;
            }
            Flash::ok(format!("Filed into {names}"))
        }
        "empty" => Flash::warn("An empty thought won’t drop — write something first"),
        "too_long" => Flash::warn("That one won’t fit — the slot takes 280 characters"),
        "updated" => Flash::ok("Idea updated"),
        "buckets_updated" => Flash::ok("Tins updated"),
        "deleted" => Flash::ok("Removed — the box is lighter by one idea"),
        "not_found" => Flash::warn("That idea isn’t in the box anymore"),
        "bucket_deleted" => Flash::ok(format!(
            "Freed {} idea(s) — the tin is gone, the ideas stay",
            payload.unwrap_or("0")
        )),
        "bucket_renamed" => Flash::ok("Tin renamed"),
        "empty_name" => Flash::warn("A tin needs a name"),
        "empty_bucket" => Flash::warn("Nothing in reach — the box is empty"),
        "dipped" => Flash::ok("Pulled one out of the box"),
        "imported" => {
            // payload is "<imported>,<skipped>"
            let (imported, skipped) = payload
                .unwrap_or("0,0")
                .split_once(',')
                .unwrap_or(("0", "0"));
            Flash::ok(format!("Imported {imported} idea(s), skipped {skipped}"))
        }
        "import_failed" => Flash::warn("That doesn’t look like a Rusty Box export"),
        "save_failed" => Flash {
            kind: FlashKind::Err,
            text: "Could not write the data file — this idea lives in memory only".into(),
        },
        _ => return None,
    };
    Some(flash)
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
    fn known_codes_resolve_with_kind_and_text() {
        let dropped = resolve("dropped", None).unwrap();
        assert_eq!(dropped.kind, FlashKind::Ok);
        assert_eq!(dropped.kind.class(), "toast-ok");
        assert_eq!(
            dropped.text,
            "Dropped into the box — file it later if you like"
        );

        let filed = resolve("filed", Some("Workshop + Side")).unwrap();
        assert_eq!(filed.text, "Filed into Workshop + Side");

        let failed = resolve("save_failed", None).unwrap();
        assert_eq!(failed.kind, FlashKind::Err);
        assert_eq!(failed.kind.class(), "toast-err");
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
            resolve("imported", Some("3,1")).unwrap().text,
            "Imported 3 idea(s), skipped 1"
        );
        assert_eq!(
            resolve("imported", None).unwrap().text,
            "Imported 0 idea(s), skipped 0"
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
