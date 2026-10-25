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

use crate::flash::{self, Flash};
use crate::form::HtmlForm;
use crate::state::{persist, SharedState};
use crate::view::{
    blob_layout, fmt_date, fmt_datetime, level_y, more_count, surface_rx, truncate_card,
    url_encode, MAX_BLOBS,
};

/// The favicon and the social-card image are compiled into the binary, so the
/// whole app still works with no files around it and no network to fetch from.
const FAVICON_SVG: &str = include_str!("../static/favicon.svg");
const OG_IMAGE_SVG: &str = include_str!("../static/og-image.svg");

/// The theme cookie: one year, server-read only (there is no JavaScript to
/// read it client-side, and nothing else on the site needs cookies).
const THEME_COOKIE: &str = "rustybox_theme";

// ---------- query / form payloads ----------

#[derive(Debug, Deserialize)]
pub struct IndexQuery {
    q: Option<String>,
    bucket: Option<String>,
    m: Option<String>,
    p: Option<String>,
    /// Theme override from the footer picker: `light`, `dark`, or empty for
    /// "follow the system". The choice lands in a one-year cookie.
    theme: Option<String>,
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
pub struct RenameForm {
    from: String,
    to: String,
}

#[derive(Debug, Deserialize)]
pub struct ImportForm {
    payload: String,
    #[serde(default)]
    mode: String,
}

/// Some POST-rendered pages still honour `?theme=` on their URL.
#[derive(Debug, Deserialize)]
pub struct ThemeQuery {
    theme: Option<String>,
}

// ---------- template view models ----------

pub struct ChipVm {
    name: String,
    color: String,
}

/// One row of the rail's tin list. The "All" pseudo-tin opens the unfiltered
/// box and carries no rename/delete tools — those belong to real tins only.
pub struct TinVm {
    name: String,
    color: String,
    count: usize,
    active: bool,
    tools: bool,
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
    flash_class: &'static str,
    /// Explicit theme for `<html data-theme=...>`; empty means "follow the
    /// system" and renders no attribute.
    theme: String,
    /// The quarantined backup path, when boot moved a corrupt data file
    /// aside. Empty on a healthy boot.
    quarantine: String,
    level_y: i64,
    surface_rx: i64,
    has_ideas: bool,
    drip_visible: bool,
    blobs: Vec<BlobVm>,
    more_text: String,
    count: usize,
    count_label: String,
    /// the rail's tin list: All + every tin with its count
    tins: Vec<TinVm>,
    tin_count: usize,
    active_bucket: String,
    q: String,
    cards: Vec<CardVm>,
    /// composer filing checkboxes: every known tin
    filing: Vec<ChipVm>,
    into: String,
    /// the search link that drops the query but keeps the tin filter
    clear_search_href: String,
    empty_title: String,
    empty_hint: String,
    dip_href: String,
}

#[derive(Template)]
#[template(path = "idea.html")]
pub struct IdeaTemplate {
    flash: String,
    flash_class: &'static str,
    theme: String,
    quarantine: String,
    editing: bool,
    id: String,
    text: String,
    source: String,
    created: String,
    edited_line: String,
    in_inbox: bool,
    buckets: Vec<CardBucketVm>,
    /// filing checkboxes for the editor, pre-checked to the idea's buckets
    filing: Vec<EditChipVm>,
    share_url: String,
    text_len: usize,
}

/// The two-step guard page for a destructive replace-import: shown instead
/// of mutating, until the same payload is confirmed.
#[derive(Template)]
#[template(path = "confirm.html")]
pub struct ConfirmTemplate {
    flash: String,
    flash_class: &'static str,
    theme: String,
    quarantine: String,
    count: usize,
    incoming: usize,
    payload: String,
}

/// The styled 404: every dead end leads back to the box.
#[derive(Template)]
#[template(path = "404.html")]
pub struct NotFoundTemplate {
    flash: String,
    flash_class: &'static str,
    theme: String,
    quarantine: String,
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
        .route("/buckets/rename", post(bucket_rename))
        .route("/dip", get(dip))
        .route("/export/json", get(export_json_route))
        .route("/export/md", get(export_md_route))
        .route("/import", post(import_route))
        .route("/import/confirm", post(import_confirm))
        .route("/static/favicon.svg", get(static_favicon))
        .route("/static/og-image.svg", get(static_og_image))
        .route("/healthz", get(healthz))
        .fallback(not_found_page)
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

fn flash_of(code: &Option<String>, payload: &Option<String>) -> Option<Flash> {
    flash::resolve(code.as_deref()?, payload.as_deref())
}

/// The theme for this render: the query override wins (that is the footer
/// picker's own transport), then the stored cookie, otherwise follow the
/// system. Only `light`/`dark` are explicit; anything else renders no
/// `data-theme` attribute and the CSS media query decides.
fn effective_theme(query_theme: &Option<String>, headers: &axum::http::HeaderMap) -> String {
    let from_query = query_theme.as_deref().map(str::trim);
    if let Some(t) = from_query {
        return match t {
            "light" | "dark" => t.to_string(),
            _ => String::new(),
        };
    }
    if let Some(raw) = headers.get(header::COOKIE).and_then(|v| v.to_str().ok()) {
        for pair in raw.split(';') {
            let mut it = pair.trim().splitn(2, '=');
            if it.next() == Some(THEME_COOKIE) {
                if let Some(value) = it.next() {
                    if value == "light" || value == "dark" {
                        return value.to_string();
                    }
                }
            }
        }
    }
    String::new()
}

/// The Set-Cookie header for a theme choice: one year for light/dark, an
/// immediate expiry for "auto" (clear it and follow the system again).
fn theme_cookie(value: &str) -> Option<String> {
    match value {
        "light" | "dark" => Some(format!(
            "{THEME_COOKIE}={value}; Max-Age=31536000; Path=/; SameSite=Lax; HttpOnly"
        )),
        "" => Some(format!(
            "{THEME_COOKIE}=; Max-Age=0; Path=/; SameSite=Lax; HttpOnly"
        )),
        _ => None,
    }
}

/// Attach a Set-Cookie header (the theme picker is the only user).
fn with_cookie(mut response: Response, cookie: Option<String>) -> Response {
    if let Some(value) = cookie {
        if let Ok(v) = axum::http::HeaderValue::from_str(&value) {
            response.headers_mut().append(header::SET_COOKIE, v);
        }
    }
    response
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

fn chip_vm(name: &str) -> ChipVm {
    ChipVm {
        name: name.to_string(),
        color: bucket_color(name).to_string(),
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
        edited: idea.edited.is_some(),
        buckets: shown,
        extra_buckets: extra,
    }
}

// ---------- handlers ----------

async fn healthz() -> &'static str {
    "ok"
}

async fn index(
    State(st): State<SharedState>,
    Query(query): Query<IndexQuery>,
    headers: axum::http::HeaderMap,
) -> Response {
    let flash_msg = flash_of(&query.m, &query.p);
    let theme = effective_theme(&query.theme, &headers);
    let cookie = query.theme.as_ref().and_then(|t| theme_cookie(t.trim()));
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
    let mut tins: Vec<TinVm> = vec![TinVm {
        name: "All".into(),
        color: bucket_color("").to_string(),
        count: store.ideas.len(),
        active: query.bucket.is_none(),
        tools: false,
        href: format!("/?{}", q_suffix(&query.q).trim_start_matches('&')),
    }];
    tins.extend(index_chips.iter().map(|b| TinVm {
        name: b.name.clone(),
        color: bucket_color(&b.name).to_string(),
        count: b.count,
        active: query.bucket.as_deref() == Some(b.name.as_str()),
        tools: true,
        href: format!("/?bucket={}{}", url_encode(&b.name), q_suffix(&query.q)),
    }));

    let filing: Vec<ChipVm> = index_chips.iter().map(|b| chip_vm(&b.name)).collect();

    let cards: Vec<CardVm> = visible.iter().map(|i| card_vm(i)).collect();

    // the box's empty states, in tin vocabulary
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
                "Open an idea, tap Edit, and file it here.".into(),
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
            "The box is empty.".into(),
            "Drop your first idea above — that’s the ritual.".into(),
        )
    };

    // "clear the search" keeps the tin filter, drops the query
    let clear_search_href = match &query.bucket {
        Some(bucket) => format!("/?bucket={}", url_encode(bucket)),
        None => "/".to_string(),
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

    let (flash_text, flash_class) = match &flash_msg {
        Some(f) => (f.text.clone(), f.kind.class()),
        None => (String::new(), ""),
    };
    let quarantine = st.quarantine.clone().unwrap_or_default();
    drop(store);
    with_cookie(
        page(&IndexTemplate {
            flash: flash_text,
            flash_class,
            theme,
            quarantine,
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
                Some(bucket) => format!("in the “{bucket}” tin"),
                None if n == 1 => "idea in the box".to_string(),
                None => "ideas in the box".to_string(),
            },
            tins,
            tin_count: index_chips.len(),
            active_bucket: query.bucket.clone().unwrap_or_default(),
            q: query.q.clone().unwrap_or_default(),
            cards,
            filing,
            into: query.bucket.clone().unwrap_or_default(),
            clear_search_href,
            empty_title,
            empty_hint,
            dip_href,
        }),
        cookie,
    )
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
    headers: axum::http::HeaderMap,
) -> Response {
    let flash_msg = flash_of(&query.m, &None);
    let theme = effective_theme(&None, &headers);
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

    let (flash_text, flash_class) = match &flash_msg {
        Some(f) => (f.text.clone(), f.kind.class()),
        None => (String::new(), ""),
    };
    page(&IdeaTemplate {
        flash: flash_text,
        flash_class,
        theme,
        quarantine: String::new(),
        editing: query.edit.is_some(),
        id: idea.id.clone(),
        text: idea.text.clone(),
        source: idea.source.clone(),
        created: fmt_datetime(idea.created),
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

async fn bucket_rename(
    State(st): State<SharedState>,
    HtmlForm(form): HtmlForm<RenameForm>,
) -> Response {
    let mut app = st.lock();
    match app.rename_bucket(&form.from, &form.to) {
        Ok(_) => {
            let failure = persist(&app);
            drop(app);
            match failure {
                Some(code) => redirect(format!("/{}", flash::redirect_query(code, None))),
                None => redirect(format!(
                    "/{}",
                    flash::redirect_query("bucket_renamed", None)
                )),
            }
        }
        Err(_) => {
            drop(app);
            redirect(format!("/{}", flash::redirect_query("empty_name", None)))
        }
    }
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
    attachment(&body, "application/json", "rusty-box.json")
}

async fn export_md_route(State(st): State<SharedState>) -> Response {
    let body = {
        let app = st.lock();
        export_markdown(&app.ideas)
    };
    attachment(&body, "text/markdown; charset=utf-8", "rusty-box.md")
}

async fn import_confirm(
    State(st): State<SharedState>,
    HtmlForm(form): HtmlForm<ImportForm>,
) -> Response {
    // The confirm step is replace-only by definition; anything else is a
    // mangled form and gets the failure flash.
    let mut app = st.lock();
    match import(&mut app.ideas, &form.payload, ImportMode::Replace) {
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

async fn static_favicon() -> Response {
    svg_response(FAVICON_SVG)
}

async fn static_og_image() -> Response {
    svg_response(OG_IMAGE_SVG)
}

fn svg_response(body: &'static str) -> Response {
    (
        [
            (header::CONTENT_TYPE, "image/svg+xml".to_string()),
            (header::CACHE_CONTROL, "public, max-age=3600".to_string()),
        ],
        body,
    )
        .into_response()
}

/// The styled 404. Status code stays 404; the copy stays honest about it.
async fn not_found_page(headers: axum::http::HeaderMap) -> Response {
    let theme = effective_theme(&None, &headers);
    let mut response = page(&NotFoundTemplate {
        flash: String::new(),
        flash_class: "",
        theme,
        quarantine: String::new(),
    });
    *response.status_mut() = axum::http::StatusCode::NOT_FOUND;
    response
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
    Query(query): Query<ThemeQuery>,
    headers: axum::http::HeaderMap,
    HtmlForm(form): HtmlForm<ImportForm>,
) -> Response {
    let mode = if form.mode == "replace" {
        ImportMode::Replace
    } else {
        ImportMode::Merge
    };

    // The destructive mode never acts on the first submit: it shows a
    // confirmation page carrying the same payload, and nothing is touched
    // until that page's form posts to /import/confirm.
    if mode == ImportMode::Replace {
        let incoming =
            serde_json::from_str::<Vec<serde_json::Value>>(&form.payload).map(|v| v.len());
        return match incoming {
            Ok(incoming) => {
                let theme = effective_theme(&query.theme, &headers);
                let count = { st.lock().ideas.len() };
                page(&ConfirmTemplate {
                    flash: String::new(),
                    flash_class: "",
                    theme,
                    quarantine: String::new(),
                    count,
                    incoming,
                    payload: form.payload,
                })
            }
            Err(_) => redirect(format!("/{}", flash::redirect_query("import_failed", None))),
        };
    }

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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderMap;

    fn cookie_header(value: &str) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert(
            header::COOKIE,
            axum::http::HeaderValue::from_str(value).unwrap(),
        );
        h
    }

    #[test]
    fn query_override_wins_over_the_cookie() {
        let headers = cookie_header("rustybox_theme=dark; other=x");
        assert_eq!(effective_theme(&Some("light".into()), &headers), "light");
        // an unknown query value means "auto": it wins by clearing
        assert_eq!(effective_theme(&Some("".into()), &headers), "");
        assert_eq!(effective_theme(&Some("neon".into()), &headers), "");
    }

    #[test]
    fn cookie_theme_is_used_when_the_query_is_silent() {
        let headers = cookie_header("a=b; rustybox_theme=dark; c=d");
        assert_eq!(effective_theme(&None, &headers), "dark");
        assert_eq!(effective_theme(&Some("light".into()), &headers), "light");
        assert_eq!(effective_theme(&None, &HeaderMap::new()), "");
        // a missing or junk cookie leaves the theme to the system
        assert_eq!(
            effective_theme(&None, &cookie_header("rustybox_theme=purple")),
            ""
        );
    }

    #[test]
    fn theme_cookie_header_matches_the_contract() {
        let year = theme_cookie("dark").unwrap();
        assert!(year.starts_with("rustybox_theme=dark;"));
        assert!(year.contains("Max-Age=31536000"));
        assert!(year.contains("SameSite=Lax"));
        let cleared = theme_cookie("").unwrap();
        assert!(cleared.contains("Max-Age=0"), "auto clears the cookie");
        assert!(theme_cookie("neon").is_none());
    }
}
