//! HTTP integration suite: the real router driven in-process with
//! `tower::ServiceExt::oneshot`. No sockets, no browser — same as the v1
//! jsdom suite's philosophy: boot the real app, drive it end to end.

use axum::body::Body;
use axum::http::header::{HeaderMap, HeaderValue};
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use ideabucket_core::Store;
use ideabucket_server::routes::router;
use ideabucket_server::state::AppState;
use tower::ServiceExt;

struct TestApp {
    app: Router,
    data: std::path::PathBuf,
    _dir: tempfile::TempDir,
}

impl TestApp {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path().join("idea-bucket.json");
        let store = Store::empty(&data);
        TestApp {
            app: router(std::sync::Arc::new(AppState::new(store))),
            data,
            _dir: dir,
        }
    }
}

async fn get(app: &Router, uri: &str) -> (StatusCode, HeaderMap, String) {
    let response = app
        .clone()
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let body = String::from_utf8(
        response
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .to_vec(),
    )
    .unwrap();
    (status, headers, body)
}

async fn post_form(app: &Router, uri: &str, form: &str) -> (StatusCode, HeaderMap, String) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(uri)
                .header("content-type", "application/x-www-form-urlencoded")
                .body(Body::from(form.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let body = String::from_utf8(
        response
            .into_body()
            .collect()
            .await
            .unwrap()
            .to_bytes()
            .to_vec(),
    )
    .unwrap();
    (status, headers, body)
}

fn location(headers: &HeaderMap) -> String {
    headers
        .get("location")
        .map(|v: &HeaderValue| v.to_str().unwrap().to_string())
        .unwrap_or_default()
}

#[tokio::test]
async fn healthz_is_ok() {
    let t = TestApp::new();
    let (status, _, body) = get(&t.app, "/healthz").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, "ok");
}

#[tokio::test]
async fn index_renders_the_bucket() {
    let t = TestApp::new();
    let (status, _, body) = get(&t.app, "/").await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("The Idea Bucket"));
    assert!(body.contains("drop a thought in. dip for old ones."));
    assert!(body.contains("ideas in the bucket"));
    assert!(body.contains("The bucket is empty."));
    assert!(body.contains("راستی"));
    // the glass starts empty: liquid at the bottom, drip hint visible
    assert!(body.contains(r#"y="325""#));
    assert!(body.contains(r#"opacity="1""#));
    assert!(body.contains("no bucket yet — ideas live in the inbox until you file them"));
}

#[tokio::test]
async fn drop_adds_an_idea_and_renders_it() {
    let t = TestApp::new();
    let (status, headers, _) = post_form(&t.app, "/drop", "text=Rust+rewrite+shipped").await;
    assert_eq!(status, StatusCode::SEE_OTHER);
    assert_eq!(location(&headers), "/?m=dropped");

    let (_, _, body) = get(&t.app, "/").await;
    assert!(body.contains("Rust rewrite shipped"));
    assert!(
        body.contains("idea in the bucket"),
        "singular counter: {body}"
    );
    // the liquid rose one notch: 325 - 1/20 * 175 = 316.25 -> 316
    assert!(body.contains(r#"y="316""#));
    // and the store was persisted
    let saved = std::fs::read_to_string(&t.data).unwrap();
    assert!(saved.contains("Rust rewrite shipped"));
}

#[tokio::test]
async fn drop_rejects_empty_text() {
    let t = TestApp::new();
    let (status, headers, _) = post_form(&t.app, "/drop", "text=%20%20").await;
    assert_eq!(status, StatusCode::SEE_OTHER);
    assert_eq!(location(&headers), "/?m=empty");
    let (_, _, body) = get(&t.app, "/?m=empty").await;
    assert!(body.contains("An idea can’t be empty"));
}

#[tokio::test]
async fn drop_rejects_text_over_the_capture_limit() {
    let t = TestApp::new();
    let long = "x".repeat(281);
    let (status, headers, _) = post_form(&t.app, "/drop", &format!("text={long}")).await;
    assert_eq!(status, StatusCode::SEE_OTHER);
    assert_eq!(location(&headers), "/?m=too_long");
}

#[tokio::test]
async fn drop_files_into_checked_buckets_and_new_buckets() {
    let t = TestApp::new();
    // a checked bucket plus a brand-new one, in one drop
    let (status, headers, _) =
        post_form(&t.app, "/drop", "text=filing+test&new_bucket=Fresh").await;
    assert_eq!(status, StatusCode::SEE_OTHER);
    assert_eq!(location(&headers), "/?m=filed&p=Fresh");

    let (_, _, body) = get(&t.app, "/").await;
    assert!(body.contains("Fresh"));
    assert!(body.contains("file into"));
}

#[tokio::test]
async fn drop_into_the_active_bucket_fallback() {
    let t = TestApp::new();
    // v1's addIdea: with an armed bucket filter and nothing checked, the drop
    // lands in the bucket you are looking at
    let (status, headers, _) = post_form(&t.app, "/drop", "text=hello&into=Work").await;
    assert_eq!(status, StatusCode::SEE_OTHER);
    assert_eq!(location(&headers), "/?m=filed&p=Work");

    let (_, _, body) = get(&t.app, "/?bucket=Work").await;
    assert!(body.contains("hello"));
}

#[tokio::test]
async fn xss_in_idea_text_is_escaped() {
    let t = TestApp::new();
    post_form(&t.app, "/drop", "text=%3Cscript%3Ealert(1)%3C%2Fscript%3E").await;
    let (_, _, body) = get(&t.app, "/").await;
    assert!(
        !body.contains("<script>alert(1)"),
        "raw script tag must not render: {body}"
    );
    assert!(
        body.contains("&#60;script&#62;"),
        "escaped opening tag: {}",
        body
    );
    assert!(body.contains("alert(1)"));

    let (_, _, body) = get(&t.app, "/").await;
    assert!(
        !body.contains("<script>alert(1)"),
        "the idea page must not render it raw either"
    );
}

#[tokio::test]
async fn search_filters_text_and_bucket_names() {
    let t = TestApp::new();
    post_form(&t.app, "/drop", "text=rust+idea&buckets=Work").await;
    post_form(&t.app, "/drop", "text=coffee+thought").await;

    let (_, _, body) = get(&t.app, "/?q=rust").await;
    assert!(body.contains("rust idea"));
    assert!(!body.contains("coffee thought"), "search narrows the list");

    // a query can find ideas by the bucket they sit in (v1 searches buckets)
    let (_, _, body) = get(&t.app, "/?q=work").await;
    assert!(body.contains("rust idea"));
    assert!(!body.contains("coffee thought"));

    // searching also narrows the glass counter
    let (_, _, body) = get(&t.app, "/?q=rust").await;
    assert!(body.contains("<strong>1</strong>"));
}

#[tokio::test]
async fn bucket_filter_and_active_chip() {
    let t = TestApp::new();
    post_form(&t.app, "/drop", "text=one&buckets=Work").await;
    post_form(&t.app, "/drop", "text=two").await;

    let (_, _, body) = get(&t.app, "/?bucket=Work").await;
    assert!(body.contains("in “Work”"));
    assert!(body.contains("chip active"));
    assert!(body.contains("one"));
    assert!(!body.contains(">two<"), "unfiled idea is filtered out");
    // the "delete this bucket" affordance appears with the active filter
    assert!(body.contains("Delete this bucket"));
    // the composer carries the armed bucket so the next drop files into it
    assert!(body.contains(r#"name="into" value="Work""#));
}

#[tokio::test]
async fn idea_page_view_mode() {
    let t = TestApp::new();
    post_form(
        &t.app,
        "/drop",
        "text=deep+link+me&source=https%3A%2F%2Fexample.com",
    )
    .await;
    // find the id from the saved store
    let saved = std::fs::read_to_string(&t.data).unwrap();
    let v: serde_json::Value = serde_json::from_str(&saved).unwrap();
    let id = v[0]["id"].as_str().unwrap().to_string();

    let (status, _, body) = get(&t.app, &format!("/idea/{id}")).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.contains("deep link me"));
    assert!(body.contains("from: https://example.com"));
    assert!(body.contains("Created today"));
    assert!(body.contains(&id));
    assert!(body.contains("In the inbox — no bucket yet. Tap Edit to file it."));
}

#[tokio::test]
async fn unknown_idea_bounces_home_with_a_flash() {
    let t = TestApp::new();
    let (status, headers, _) = get(&t.app, "/idea/nope").await;
    assert_eq!(status, StatusCode::SEE_OTHER);
    assert_eq!(location(&headers), "/?m=not_found");
    let (_, _, body) = get(&t.app, "/?m=not_found").await;
    assert!(body.contains("That idea isn’t here anymore"));
}

#[tokio::test]
async fn edit_flow_updates_text_and_marks_edited() {
    let t = TestApp::new();
    post_form(&t.app, "/drop", "text=before+edit").await;
    let saved = std::fs::read_to_string(&t.data).unwrap();
    let id: String = serde_json::from_str::<serde_json::Value>(&saved).unwrap()[0]["id"]
        .as_str()
        .unwrap()
        .to_string();

    // the edit form is prefilled and the buckets pre-checked
    let (_, _, form) = get(&t.app, &format!("/idea/{id}?edit=1")).await;
    assert!(form.contains("before edit"));
    assert!(form.contains("ideaEditArea"));

    let (status, headers, _) = post_form(
        &t.app,
        &format!("/idea/{id}/edit"),
        "text=after+edit&new_bucket=Work",
    )
    .await;
    assert_eq!(status, StatusCode::SEE_OTHER);
    assert_eq!(location(&headers), format!("/idea/{id}?m=updated"));

    let (_, _, page) = get(&t.app, &format!("/idea/{id}")).await;
    assert!(page.contains("after edit"));
    assert!(page.contains("Edited "), "edit stamp shows: {page}");

    let (_, _, list) = get(&t.app, "/").await;
    assert!(list.contains("· edited"));
}

#[tokio::test]
async fn edit_with_only_bucket_changes_reports_buckets_updated() {
    let t = TestApp::new();
    post_form(&t.app, "/drop", "text=keep+text").await;
    let saved = std::fs::read_to_string(&t.data).unwrap();
    let id: String = serde_json::from_str::<serde_json::Value>(&saved).unwrap()[0]["id"]
        .as_str()
        .unwrap()
        .to_string();

    let (_, headers, _) = post_form(
        &t.app,
        &format!("/idea/{id}/edit"),
        "text=keep+text&buckets=Work",
    )
    .await;
    assert_eq!(location(&headers), format!("/idea/{id}?m=buckets_updated"));

    let (_, _, page) = get(&t.app, &format!("/idea/{id}")).await;
    assert!(page.contains("Work"));
    assert!(page.contains("keep text"));
}

#[tokio::test]
async fn edit_rejects_empty_text() {
    let t = TestApp::new();
    post_form(&t.app, "/drop", "text=keep+text").await;
    let saved = std::fs::read_to_string(&t.data).unwrap();
    let id: String = serde_json::from_str::<serde_json::Value>(&saved).unwrap()[0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let (_, headers, _) = post_form(&t.app, &format!("/idea/{id}/edit"), "text=%20").await;
    assert_eq!(location(&headers), format!("/idea/{id}?m=empty"));
}

#[tokio::test]
async fn delete_removes_the_idea() {
    let t = TestApp::new();
    post_form(&t.app, "/drop", "text=delete+me").await;
    let saved = std::fs::read_to_string(&t.data).unwrap();
    let id: String = serde_json::from_str::<serde_json::Value>(&saved).unwrap()[0]["id"]
        .as_str()
        .unwrap()
        .to_string();

    let (status, headers, _) = post_form(&t.app, &format!("/idea/{id}/delete"), "").await;
    assert_eq!(status, StatusCode::SEE_OTHER);
    assert_eq!(location(&headers), "/?m=deleted");

    let (_, _, body) = get(&t.app, "/").await;
    assert!(!body.contains("delete me"));
    let (_, headers, _) = get(&t.app, &format!("/idea/{id}")).await;
    assert_eq!(location(&headers), "/?m=not_found");
}

#[tokio::test]
async fn bucket_delete_unfiles_but_never_deletes() {
    let t = TestApp::new();
    post_form(&t.app, "/drop", "text=survivor&buckets=Work").await;

    let (status, headers, _) = post_form(&t.app, "/buckets/delete", "name=Work").await;
    assert_eq!(status, StatusCode::SEE_OTHER);
    assert_eq!(location(&headers), "/?m=bucket_deleted&p=1");

    let (_, _, body) = get(&t.app, "/").await;
    assert!(body.contains("survivor"), "the idea survives its bucket");
    assert!(!body.contains(">Work<"), "the bucket is gone");
}

#[tokio::test]
async fn dip_from_a_pool_and_from_an_empty_bucket() {
    let t = TestApp::new();
    // empty pool: v1's toast
    let (status, headers, _) = get(&t.app, "/dip").await;
    assert_eq!(status, StatusCode::SEE_OTHER);
    assert_eq!(location(&headers), "/?m=empty_bucket");

    post_form(&t.app, "/drop", "text=first").await;
    post_form(&t.app, "/drop", "text=second").await;
    let (status, headers, _) = get(&t.app, "/dip").await;
    assert_eq!(status, StatusCode::SEE_OTHER);
    let loc = location(&headers);
    assert!(
        loc.starts_with("/idea/"),
        "dip lands on the idea page: {loc}"
    );
    assert!(loc.contains("m=dipped"));

    // dipping within a bucket filter only picks from that bucket
    post_form(&t.app, "/drop", "text=filed&buckets=Work").await;
    let (_, headers, _) = get(&t.app, "/dip?bucket=Work").await;
    let loc = location(&headers);
    assert!(loc.starts_with("/idea/"), "{loc}");
    let (_, _, page) = get(&t.app, &loc).await;
    assert!(page.contains("filed"));
}

#[tokio::test]
async fn json_export_is_an_attachment() {
    let t = TestApp::new();
    post_form(&t.app, "/drop", "text=exported").await;
    let (status, headers, body) = get(&t.app, "/export/json").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers.get("content-type").unwrap(), "application/json");
    assert_eq!(
        headers.get("content-disposition").unwrap(),
        "attachment; filename=\"idea-bucket.json\""
    );
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v[0]["text"], "exported");
    assert!(
        v[0].get("edited").is_none(),
        "v1 shape: no edited key until edited"
    );
}

#[tokio::test]
async fn markdown_export_has_bucket_sections() {
    let t = TestApp::new();
    // one idea filed into two buckets: it appears under each, cross-noted
    post_form(
        &t.app,
        "/drop",
        "text=shared+idea&buckets=Work&buckets=Side",
    )
    .await;
    post_form(&t.app, "/drop", "text=loose").await;

    let (status, headers, body) = get(&t.app, "/export/md").await;
    assert_eq!(status, StatusCode::OK);
    assert!(headers
        .get("content-disposition")
        .unwrap()
        .to_str()
        .unwrap()
        .contains("idea-bucket.md"));
    assert!(body.starts_with("# The Idea Bucket"));
    assert!(body.contains("## Work (1)"));
    assert!(body.contains("## Side (1)"));
    assert!(body.contains("_(also in Work)_"));
    assert!(body.contains("_(also in Side)_"));
    assert!(body.contains("## Unfiled (1)"));
}

#[tokio::test]
async fn import_accepts_a_v1_export() {
    let t = TestApp::new();
    let payload = r##"[{"id":"aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee","text":"from the browser","color":"#f0b429","created":1700000000000,"lastViewed":1700000000000,"buckets":["Work"],"source":""},{"text":"legacy bare entry"}]"##;
    let encoded = urlencode(payload);
    let (status, headers, _) =
        post_form(&t.app, "/import", &format!("payload={encoded}&mode=merge")).await;
    assert_eq!(status, StatusCode::SEE_OTHER);
    assert_eq!(location(&headers), "/?m=imported&p=2%2C0");

    let (_, _, body) = get(&t.app, "/").await;
    assert!(body.contains("from the browser"));
    assert!(body.contains("legacy bare entry"));
}

#[tokio::test]
async fn import_replace_wipes_and_import_failed_reports() {
    let t = TestApp::new();
    post_form(&t.app, "/drop", "text=existing").await;

    let (status, headers, _) = post_form(
        &t.app,
        "/import",
        "payload=%5B%7B%22text%22%3A%22only%20me%22%7D%5D&mode=replace",
    )
    .await;
    assert_eq!(status, StatusCode::SEE_OTHER);
    assert_eq!(location(&headers), "/?m=imported&p=1%2C0");
    let (_, _, body) = get(&t.app, "/").await;
    assert!(body.contains("only me"));
    assert!(
        !body.contains(">existing<"),
        "the replaced idea's card is gone"
    );

    let (_, headers, _) = post_form(&t.app, "/import", "payload=not+json").await;
    assert_eq!(location(&headers), "/?m=import_failed");
    let (_, headers, _) =
        post_form(&t.app, "/import", "payload=%7B%22just%22%3A%22object%22%7D").await;
    assert_eq!(location(&headers), "/?m=import_failed");
}

#[tokio::test]
async fn unknown_flash_codes_render_nothing() {
    let t = TestApp::new();
    let (_, _, body) = get(&t.app, "/?m=made-up-code").await;
    assert!(!body.contains("toast show"));
}

#[tokio::test]
async fn chip_links_url_encode_bucket_names() {
    let t = TestApp::new();
    post_form(&t.app, "/drop", "text=one&new_bucket=side+projects").await;
    let (_, _, body) = get(&t.app, "/").await;
    assert!(body.contains("href=\"/?bucket=side%20projects\""), "{body}");
    // and the encoded link actually works
    let (_, _, filtered) = get(&t.app, "/?bucket=side%20projects").await;
    assert!(filtered.contains("in “side projects”"));
    assert!(filtered.contains("one"));
}

#[tokio::test]
async fn corrupt_store_is_quarantined_at_boot() {
    let dir = tempfile::tempdir().unwrap();
    let data = dir.path().join("idea-bucket.json");
    std::fs::write(&data, "{truncated").unwrap();
    let store = Store::load(&data).unwrap();
    let app = router(std::sync::Arc::new(AppState::new(store)));
    let (_, _, body) = get(&app, "/").await;
    assert!(body.contains("The bucket is empty."));
    assert!(dir.path().join("idea-bucket.json.corrupt-backup").exists());
}

/// Minimal application/x-www-form-urlencoded encoder for test payloads.
fn urlencode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'*' => {
                out.push(b as char)
            }
            b' ' => out.push_str("%20"),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}
