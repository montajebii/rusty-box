//! HTTP handlers — one per v1 behavior, all server-rendered, zero JavaScript.
//!
//! Every mutation responds with a redirect (`/?m=<flash-code>` or
//! `/idea/<id>?m=<flash-code>`); the next GET renders the message as a
//! fading toast. Filters (`bucket`, `q`) travel in the query string, so any
//! view is bookmarkable.

use askama::Template;
use axum::extract::{Path, Query, State};
use axum::http::header;
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::routing::{get, post};
use axum::Router;
use serde::Deserialize;

use rustybox_core::{
    bucket_color, bucket_index, export_json, export_markdown, import, BucketEntry, EditError,
    Filter, ImportMode, TextError,
};

use crate::flash;
use crate::form::HtmlForm;
use crate::state::{persist, SharedState};
use crate::view::{
    age_line, blob_layout, fmt_date, fmt_datetime, level_y, more_count, surface_rx, truncate_card,
    url_encode, MAX_BLOBS,
};

// ---------- query / form payloads ----------

#[derive(Debug, Deserialize)]
pub struct IndexQuery {
    q: Option<String>,
    bucket: Option<String>,
    m: Option<String>,
    p: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct IdeaQuery {
    edit: Option<String>,
    m: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DipQuery {
    q: Option<String>,
    bucket: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DropForm {
    text: String,
    #[serde(default)]
    buckets: Vec<String>,
    #[serde(default)]
    new_bucket: String,
    #[serde(default)]
    source: String,
    /// Active bucket filter at capture time — v1 files the drop into it when
    /// nothing else is armed (`addIdea`'s activeBucket fallback).
    #[serde(default)]
    into: String,
}

#[derive(Debug, Deserialize)]
pub struct EditForm {
    text: String,
    #[serde(default)]
    buckets: Vec<String>,
    #[serde(default)]
    new_bucket: String,
}

#[derive(Debug, Deserialize)]
pub struct BucketForm {
    name: String,
}

#[derive(Debug, Deserialize)]
pub struct ImportForm {
    payload: String,
    #[serde(default)]
    mode: String,
}

// ---------- template view models ----------

pub struct ChipVm {
    name: String,
    color: String,
    count: usize,
    active: bool,
    href: String,
}

/// Composer checkbox in the idea editor; `checked` mirrors where the idea is
/// currently filed.
pub struct EditChipVm {
    name: String,
    color: String,
    checked: bool,
}

pub struct CardBucketVm {
    name: String,
    color: String,
}

pub struct CardVm {
    id: String,
    text: String,
    color: String,
    date: String,
    edited: bool,
    buckets: Vec<CardBucketVm>,
    extra_buckets: usize,
}

pub struct BlobVm {
    cx: i64,
    cy: i64,
    r: i64,
    color: String,
}

#[derive(Template)]
#[template(path = "index.html")]
pub struct IndexTemplate {
    flash: String,
    level_y: i64,
    surface_rx: i64,
    has_ideas: bool,
    drip_visible: bool,
    blobs: Vec<BlobVm>,
    more_text: String,
    count: usize,
    count_label: String,
    /// filter-bar chips: All + every bucket with its count
    chips: Vec<ChipVm>,
    active_bucket: String,
    q: String,
    cards: Vec<CardVm>,
    /// composer filing checkboxes: every known bucket
    filing: Vec<ChipVm>,
    into: String,
    empty_title: String,
    empty_hint: String,
    dip_href: String,
}

#[derive(Template)]
#[template(path = "idea.html")]
pub struct IdeaTemplate {
    flash: String,
    editing: bool,
    id: String,
    text: String,
    color: String,
    source: String,
    created: String,
    age: String,
    edited_line: String,
    in_inbox: bool,
    buckets: Vec<CardBucketVm>,
    /// filing checkboxes for the editor, pre-checked to the idea's buckets
    filing: Vec<EditChipVm>,
    share_url: String,
    text_len: usize,
}

// ---------- router ----------

pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/", get(index))
        .route("/drop", post(drop_idea))
        .route("/idea/{id}", get(idea_page))
        .route("/idea/{id}/edit", post(idea_edit))
        .route("/idea/{id}/delete", post(idea_delete))
        .route("/buckets/delete", post(bucket_delete))
        .route("/dip", get(dip))
        .route("/export/json", get(export_json_route))
        .route("/export/md", get(export_md_route))
        .route("/import", post(import_route))
        .route("/healthz", get(healthz))
        .with_state(state)
}

// ---------- helpers ----------

fn page<T: Template>(template: &T) -> Response {
    match template.render() {
        Ok(body) => Html(body).into_response(),
        Err(e) => {
            eprintln!("rustybox: template render failed: {e}");
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                "render error",
            )
                .into_response()
        }
    }
}

fn redirect(path: String) -> Response {
    Redirect::to(&path).into_response()
}

fn flash_of(code: &Option<String>, payload: &Option<String>) -> Option<String> {
    flash::resolve(code.as_deref()?, payload.as_deref())
}

fn filter_from(q: &Option<String>, bucket: &Option<String>) -> Filter {
    Filter {
        bucket: bucket.clone(),
        q: q.clone(),
    }
}

fn q_suffix(q: &Option<String>) -> String {
    match q {
        Some(q) if !q.trim().is_empty() => format!("&q={}", url_encode(q)),
        _ => String::new(),
    }
}

fn chip_vm(name: &str, count: usize, active: bool, q: &Option<String>) -> ChipVm {
    // Clicking the active chip clears the bucket filter (v1's toggle);
    // clicking any other chip filters while keeping the search term.
    let href = if active {
        format!("/?{}", q_suffix(q).trim_start_matches('&'))
    } else {
        format!("/?bucket={}{}", url_encode(name), q_suffix(q))
    };
    ChipVm {
        name: name.to_string(),
        color: bucket_color(name).to_string(),
        count,
        active,
        href,
    }
}

fn card_vm(idea: &rustybox_core::Idea) -> CardVm {
    // v1's CARD_CHIPS: a card shows at most two bucket chips, then "+N".
    const CARD_CHIPS: usize = 2;
    let shown: Vec<CardBucketVm> = idea
        .buckets
        .iter()
        .take(CARD_CHIPS)
        .map(|b| CardBucketVm {
            name: b.clone(),
            color: bucket_color(b).to_string(),
        })
        .collect();
    let extra = idea.buckets.len().saturating_sub(CARD_CHIPS);
    CardVm {
        id: idea.id.clone(),
        text: truncate_card(&idea.text),
        color: idea.color.clone(),
        date: fmt_date(idea.created),
        edited: idea.edited.is_some(),
        buckets: shown,
        extra_buckets: extra,
    }
}

// ---------- handlers ----------

async fn healthz() -> &'static str {
    "ok"
}

async fn index(State(st): State<SharedState>, Query(query): Query<IndexQuery>) -> Response {
    let flash_msg = flash_of(&query.m, &query.p);
    let filter = filter_from(&query.q, &query.bucket);
    let store = st.lock();
    let visible = store.visible(&filter);
    let n = visible.len();

    let blobs: Vec<BlobVm> = blob_layout(n)
        .into_iter()
        .zip(visible.iter().take(MAX_BLOBS))
        .map(|(blob, idea)| BlobVm {
            color: idea.color.clone(),
            cx: blob.cx,
            cy: blob.cy,
            r: blob.r,
        })
        .collect();

    let index_chips: Vec<BucketEntry> = bucket_index(&store.ideas);
    let mut chips: Vec<ChipVm> = vec![ChipVm {
        name: "All".into(),
        color: bucket_color("").to_string(),
        count: store.ideas.len(),
        active: query.bucket.is_none(),
        href: format!("/?{}", q_suffix(&query.q).trim_start_matches('&')),
    }];
    chips.extend(index_chips.iter().map(|b| {
        chip_vm(
            &b.name,
            b.count,
            query.bucket.as_deref() == Some(b.name.as_str()),
            &query.q,
        )
    }));

    let filing: Vec<ChipVm> = index_chips
        .iter()
        .map(|b| chip_vm(&b.name, b.count, false, &None))
        .collect();

    let cards: Vec<CardVm> = visible.iter().map(|i| card_vm(i)).collect();

    // v1's drawer empty states
    let q_present = query.q.as_deref().is_some_and(|q| !q.trim().is_empty());
    let (empty_title, empty_hint) = if let Some(bucket) = &query.bucket {
        if q_present {
            (
                format!(
                    "Nothing in “{bucket}” matches “{}”",
                    query.q.as_deref().unwrap_or_default()
                ),
                String::new(),
            )
        } else {
            (
                format!("Nothing in “{bucket}” yet."),
                "Open an idea and tap Edit to file it here.".into(),
            )
        }
    } else if q_present {
        (
            format!(
                "Nothing matches “{}”",
                query.q.as_deref().unwrap_or_default()
            ),
            String::new(),
        )
    } else {
        (
            "The bucket is empty.".into(),
            "Drop your first idea above.".into(),
        )
    };

    // the dip link dips from what you are looking at (v1 dips from the
    // visible pool), so it carries the current filter
    let mut dip_href = String::from("/dip");
    if query.bucket.is_some() || q_present {
        dip_href.push('?');
        if let Some(bucket) = &query.bucket {
            dip_href.push_str(&format!("bucket={}", url_encode(bucket)));
        }
        dip_href.push_str(&q_suffix(&query.q));
    }

    page(&IndexTemplate {
        flash: flash_msg.unwrap_or_default(),
        level_y: level_y(n) as i64,
        surface_rx: surface_rx(n) as i64,
        has_ideas: n > 0,
        drip_visible: n == 0,
        blobs,
        more_text: more_count(n)
            .map(|m| format!("+{m} more…"))
            .unwrap_or_default(),
        count: n,
        count_label: match &query.bucket {
            Some(bucket) => format!("in “{bucket}”"),
            None if n == 1 => "idea in the bucket".to_string(),
            None => "ideas in the bucket".to_string(),
        },
        chips,
        active_bucket: query.bucket.clone().unwrap_or_default(),
        q: query.q.clone().unwrap_or_default(),
        cards,
        filing,
        into: query.bucket.clone().unwrap_or_default(),
        empty_title,
        empty_hint,
        dip_href,
    })
}

async fn drop_idea(State(st): State<SharedState>, HtmlForm(form): HtmlForm<DropForm>) -> Response {
    let mut buckets = form.buckets;
    let new_bucket = form.new_bucket.trim().to_string();
    if !new_bucket.is_empty() {
        buckets.push(new_bucket);
    }
    // v1's addIdea: an armed bucket filter receives the drop when nothing
    // else is filed.
    let into = form.into.trim().to_string();
    if buckets.iter().all(|b| b.trim().is_empty()) && !into.is_empty() {
        buckets.push(into);
    }

    let mut app = st.lock();
    let outcome = app.add(&form.text, &buckets, &form.source);
    match outcome {
        Err(e) => {
            let code = match e {
                TextError::Empty => "empty",
                TextError::TooLong { .. } => "too_long",
            };
            redirect(format!("/{}", flash::redirect_query(code, None)))
        }
        Ok(idea) => {
            let names = idea.buckets.clone();
            let failure = persist(&app);
            drop(app);
            match failure {
                Some(code) => redirect(format!("/{}", flash::redirect_query(code, None))),
                None if !names.is_empty() => redirect(format!(
                    "/{}",
                    flash::redirect_query("filed", Some(&names.join(" + ")))
                )),
                None => redirect(format!("/{}", flash::redirect_query("dropped", None))),
            }
        }
    }
}

async fn idea_page(
    State(st): State<SharedState>,
    Path(id): Path<String>,
    Query(query): Query<IdeaQuery>,
) -> Response {
    let flash_msg = flash_of(&query.m, &None);
    let store = st.lock();
    let idea = match store.get(&id) {
        Some(idea) => idea.clone(),
        None => {
            drop(store);
            return redirect(format!("/{}", flash::redirect_query("not_found", None)));
        }
    };
    // every known bucket, checked where this idea already sits
    let filing: Vec<EditChipVm> = {
        let mut names: Vec<String> = bucket_index(&store.ideas)
            .into_iter()
            .map(|b| b.name)
            .collect();
        for b in &idea.buckets {
            if !names.contains(b) {
                names.push(b.clone());
            }
        }
        names
            .iter()
            .map(|name| EditChipVm {
                name: name.clone(),
                color: bucket_color(name).to_string(),
                checked: idea.buckets.contains(name),
            })
            .collect()
    };
    drop(store);

    page(&IdeaTemplate {
        flash: flash_msg.unwrap_or_default(),
        editing: query.edit.is_some(),
        id: idea.id.clone(),
        text: idea.text.clone(),
        color: idea.color.clone(),
        source: idea.source.clone(),
        created: fmt_datetime(idea.created),
        age: age_line(idea.created, rustybox_core::model::now_ms()),
        edited_line: idea.edited.map(fmt_date).unwrap_or_default(),
        in_inbox: idea.buckets.is_empty(),
        buckets: idea
            .buckets
            .iter()
            .map(|b| CardBucketVm {
                name: b.clone(),
                color: bucket_color(b).to_string(),
            })
            .collect(),
        filing,
        share_url: format!("/idea/{}", idea.id),
        text_len: idea.text.chars().count(),
    })
}

async fn idea_edit(
    State(st): State<SharedState>,
    Path(id): Path<String>,
    HtmlForm(form): HtmlForm<EditForm>,
) -> Response {
    let mut buckets = form.buckets;
    let new_bucket = form.new_bucket.trim().to_string();
    if !new_bucket.is_empty() {
        buckets.push(new_bucket);
    }

    let mut app = st.lock();
    match app.edit(&id, &form.text, &buckets) {
        Ok(report) => {
            let failure = persist(&app);
            drop(app);
            match failure {
                Some(code) => redirect(format!("/{}", flash::redirect_query(code, None))),
                None if report.text_changed => {
                    redirect(format!("/idea/{}?m=updated", url_encode(&id)))
                }
                None => redirect(format!("/idea/{}?m=buckets_updated", url_encode(&id))),
            }
        }
        Err(EditError::NotFound) => {
            redirect(format!("/{}", flash::redirect_query("not_found", None)))
        }
        Err(EditError::Text(TextError::Empty)) => {
            redirect(format!("/idea/{}?m=empty", url_encode(&id)))
        }
        Err(EditError::Text(TextError::TooLong { .. })) => {
            redirect(format!("/idea/{}?m=too_long", url_encode(&id)))
        }
    }
}

async fn idea_delete(State(st): State<SharedState>, Path(id): Path<String>) -> Response {
    let mut app = st.lock();
    app.delete(&id);
    let code = persist(&app).unwrap_or("deleted");
    drop(app);
    redirect(format!("/{}", flash::redirect_query(code, None)))
}

async fn bucket_delete(
    State(st): State<SharedState>,
    HtmlForm(form): HtmlForm<BucketForm>,
) -> Response {
    let mut app = st.lock();
    let members = app.delete_bucket(&form.name);
    let code = persist(&app).unwrap_or("bucket_deleted");
    drop(app);
    redirect(format!(
        "/{}",
        flash::redirect_query(code, Some(&members.to_string()))
    ))
}

async fn dip(State(st): State<SharedState>, Query(query): Query<DipQuery>) -> Response {
    let filter = filter_from(&query.q, &query.bucket);
    let mut app = st.lock();
    match app.dip(&filter, &mut rand::rng()) {
        Some(idea) => {
            let id = idea.id.clone();
            let failure = persist(&app);
            drop(app);
            match failure {
                Some(code) => redirect(format!("/{}", flash::redirect_query(code, None))),
                None => redirect(format!("/idea/{}?m=dipped", url_encode(&id))),
            }
        }
        None => {
            // v1's dip toast on an empty pool, back on the home page
            redirect(format!("/{}", flash::redirect_query("empty_bucket", None)))
        }
    }
}

async fn export_json_route(State(st): State<SharedState>) -> Response {
    let body = {
        let app = st.lock();
        export_json(&app.ideas)
    };
    attachment(&body, "application/json", "idea-bucket.json")
}

async fn export_md_route(State(st): State<SharedState>) -> Response {
    let body = {
        let app = st.lock();
        export_markdown(&app.ideas)
    };
    attachment(&body, "text/markdown; charset=utf-8", "idea-bucket.md")
}

fn attachment(body: &str, content_type: &str, filename: &str) -> Response {
    (
        [
            (header::CONTENT_TYPE, content_type.to_string()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{filename}\""),
            ),
        ],
        body.to_string(),
    )
        .into_response()
}

async fn import_route(
    State(st): State<SharedState>,
    HtmlForm(form): HtmlForm<ImportForm>,
) -> Response {
    let mode = if form.mode == "replace" {
        ImportMode::Replace
    } else {
        ImportMode::Merge
    };
    let mut app = st.lock();
    match import(&mut app.ideas, &form.payload, mode) {
        Ok(report) => {
            let failure = persist(&app);
            drop(app);
            match failure {
                Some(code) => redirect(format!("/{}", flash::redirect_query(code, None))),
                None => redirect(format!(
                    "/{}",
                    flash::redirect_query(
                        "imported",
                        Some(&format!("{},{}", report.imported, report.skipped))
                    )
                )),
            }
        }
        Err(_) => {
            drop(app);
            redirect(format!("/{}", flash::redirect_query("import_failed", None)))
        }
    }
}
