//! Route tests for the kiosk, driven through [`super::handle`].
//!
//! Expected statuses, content types and bodies come from the Rails app run
//! from a copy of `~/Projects/kiosk` (commit 7c8be77) and from live GETs.

use super::*;
use crate::config::{BackendConfig, DatabaseConfig, Logger};
use crate::db::Pool;
use crate::stores::{LockoutStore, RateLimitStore};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

static SEQ: AtomicU64 = AtomicU64::new(0);

const CHROME_120: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";
const CHROME_119: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/119.0.0.0 Safari/537.36";

/// State over a fresh database and a fake `dist/`, built without reading env.
fn state() -> AppState {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("kiosk-routes-{}-{n}", std::process::id()));
    let dist = dir.join("dist");
    std::fs::create_dir_all(dist.join("images")).expect("dist");
    for (name, body) in [
        ("index.html", "<!doctype html><title>Frankie Food Court Kiosk</title>"),
        ("404.html", "not found page"),
        ("400.html", "bad request page"),
        ("422.html", "rejected page"),
        ("500.html", "error page"),
        ("406-unsupported-browser.html", "unsupported browser page"),
        ("robots.txt", "# robots"),
        ("icon.svg", "<svg/>"),
        ("images/frankie-logo.png", "png"),
    ] {
        std::fs::write(dist.join(name), body).expect("write");
    }
    let db_path = dir.join("Kiosk.db");
    let _ = std::fs::remove_file(&db_path);
    let pool = Pool::open(&db_path.to_string_lossy(), 2).expect("pool");
    let st = AppState {
        cfg: BackendConfig {
            static_dir: "dist".into(),
            database: DatabaseConfig {
                db: "Kiosk".into(),
                db_type: "sqlite".into(),
                connection_string: db_path.to_string_lossy().into_owned(),
            },
        },
        log: Logger::new(true),
        pool,
        lockout: LockoutStore::new(),
        auth_rate: RateLimitStore::new(),
        stripe: None,
        stripe_lookup_keys: Vec::new(),
        stripe_endpoint_secret: None,
        jwt_secret: None,
        cors_origins: Vec::new(),
        static_dir: PathBuf::from(&dist),
        port: 0,
        free_usage_limit: 20,
        prod: true,
    };
    prepare(&st).expect("prepare");
    st
}

/// A request from a fresh client IP, so the process-wide `POST /orders`
/// limiter never carries over between tests.
fn req(method: &str, path: &str) -> Request {
    let (raw, query) = path.split_once('?').unwrap_or((path, ""));
    let mut r = Request::for_test(method, raw);
    r.query = query.to_string();
    r.peer_ip = format!("test-{}", SEQ.fetch_add(1, Ordering::Relaxed));
    r.set_test_header("user-agent", CHROME_120);
    r.set_test_header("host", "kiosk.bixbyapps.com");
    r
}

fn post_json(path: &str, body: &str, origin: Option<&str>) -> Request {
    let mut r = req("POST", path);
    r.set_test_header("content-type", "application/json");
    if let Some(o) = origin {
        r.set_test_header("origin", o);
    }
    r.set_test_body(body.as_bytes().to_vec());
    r
}

fn header<'a>(res: &'a Response, name: &str) -> Option<&'a str> {
    res.headers.iter().find(|(k, _)| k.eq_ignore_ascii_case(name)).map(|(_, v)| v.as_str())
}

fn body(res: &Response) -> String {
    String::from_utf8_lossy(&res.body).into_owned()
}

#[test]
fn root_serves_kiosk_page() {
    let res = handle(&state(), req("GET", "/"));
    assert_eq!((res.status, header(&res, "content-type")), (200, Some("text/html; charset=utf-8")));
    assert!(body(&res).contains("Frankie Food Court Kiosk"));
}

#[test]
fn root_is_uncached_with_etag() {
    let res = handle(&state(), req("GET", "/?a=b"));
    assert_eq!(header(&res, "cache-control"), Some(DYNAMIC_CACHE));
    assert!(header(&res, "etag").is_some_and(|e| e.starts_with("W/\"")));
}

#[test]
fn matching_etag_answers_304() {
    let st = state();
    let first = handle(&st, req("GET", "/menu_items"));
    let mut again = req("GET", "/menu_items");
    again.set_test_header("if-none-match", header(&first, "etag").expect("etag"));
    let res = handle(&st, again);
    assert_eq!((res.status, res.body.len()), (304, 0));
}

#[test]
fn menu_items_is_rails_json() {
    let res = handle(&state(), req("GET", "/menu_items"));
    assert_eq!((res.status, header(&res, "content-type")), (200, Some("application/json; charset=utf-8")));
    assert!(body(&res).starts_with(r#"[{"id":1,"name":"Hotdog \u0026 Soda","category":null,"created_at":""#));
}

#[test]
fn menu_items_accepts_any_format_and_slashes() {
    let st = state();
    for path in ["/menu_items.json", "/menu_items.xml", "/menu_items/", "//menu_items", "/menu_items.json/"] {
        assert_eq!(handle(&st, req("GET", path)).status, 200, "{path}");
    }
}

#[test]
fn menu_items_unknown_shapes_are_404() {
    let st = state();
    for path in ["/MENU_ITEMS", "/menu_items/1", "/menu_items/.json", "/menu_items.a.b", "/%6Benu_items"] {
        assert_eq!(handle(&st, req("GET", path)).status, 404, "{path}");
    }
}

#[test]
fn up_is_not_routed_like_rails() {
    let res = handle(&state(), req("GET", "/up"));
    assert_eq!((res.status, body(&res).as_str()), (404, "not found page"));
}

#[test]
fn not_found_uses_rails_page() {
    let res = handle(&state(), req("GET", "/nope"));
    assert_eq!((res.status, header(&res, "content-type")), (404, Some("text/html; charset=UTF-8")));
}

#[test]
fn template_account_api_is_closed() {
    let st = state();
    for (m, p) in [("POST", "/api/signup"), ("POST", "/api/signin"), ("GET", "/api/me"), ("POST", "/api/checkout")] {
        assert_eq!(handle(&st, req(m, p)).status, 404, "{m} {p}");
    }
}

#[test]
fn health_passes_through_to_template() {
    let res = handle(&state(), req("GET", "/api/health"));
    assert_eq!(res.status, 200);
}

#[test]
fn static_files_and_html_fallback() {
    let st = state();
    assert_eq!(handle(&st, req("GET", "/robots.txt")).status, 200);
    assert_eq!(handle(&st, req("GET", "/images/frankie-logo.png")).status, 200);
    let page = handle(&st, req("GET", "/404"));
    assert_eq!((page.status, body(&page).as_str()), (200, "not found page"));
    assert_eq!(header(&page, "cache-control"), Some(STATIC_CACHE));
}

#[test]
fn static_without_html_fallback_is_404() {
    let st = state();
    assert_eq!(handle(&st, req("GET", "/images/frankie-logo")).status, 404);
    assert_eq!(handle(&st, req("GET", "/images/")).status, 404);
}

#[test]
fn kiosk_html_is_the_kiosk_page() {
    let st = state();
    for path in ["/kiosk.html", "/kiosk"] {
        let res = handle(&st, req("GET", path));
        assert!(res.status == 200 && body(&res).contains("Frankie"), "{path}");
    }
}

#[test]
fn vite_entry_is_not_public() {
    let st = state();
    assert_eq!(handle(&st, req("GET", "/index.html")).status, 404);
}

#[test]
fn old_browser_gets_406_on_actions() {
    let mut r = req("GET", "/");
    r.headers = crate::http::Headers::from_pairs(vec![("user-agent".into(), CHROME_119.into())]);
    let res = handle(&state(), r);
    assert_eq!((res.status, body(&res).as_str()), (406, "unsupported browser page"));
}

#[test]
fn old_browser_still_gets_static_files() {
    let mut r = req("GET", "/robots.txt");
    r.headers = crate::http::Headers::from_pairs(vec![("user-agent".into(), CHROME_119.into())]);
    assert_eq!(handle(&state(), r).status, 200);
}

#[test]
fn post_orders_without_origin_is_422_like_rails() {
    let res = handle(&state(), post_json("/orders", r#"{"total":"9.99"}"#, None));
    assert_eq!((res.status, body(&res).as_str()), (422, "rejected page"));
}

#[test]
fn post_orders_cross_origin_is_422() {
    let res = handle(&state(), post_json("/orders", r#"{"total":"9.99"}"#, Some("https://evil.example")));
    assert_eq!(res.status, 422);
}

#[test]
fn post_orders_same_origin_creates_order() {
    let st = state();
    let res = handle(&st, post_json("/orders", r#"{"total":"9.99","items":[{"id":1,"qty":2}]}"#, Some("https://kiosk.bixbyapps.com")));
    assert_eq!((res.status, body(&res).as_str()), (201, r#"{"message":"Order created successfully","order_id":1}"#));
    let rows = st
        .pool
        .with(|db| db.query("SELECT total, items FROM orders WHERE id = 1", &[]))
        .expect("query");
    assert_eq!(rows[0].get("total"), Some(&crate::db::Value::Real(9.99)));
    assert_eq!(rows[0].text("items"), Some(r#"[{"id":1,"qty":2}]"#));
}

#[test]
fn post_orders_sec_fetch_site_counts_as_same_origin() {
    let mut r = post_json("/orders", "{}", None);
    r.set_test_header("sec-fetch-site", "same-origin");
    assert_eq!(handle(&state(), r).status, 201);
}

#[test]
fn post_orders_form_encoded() {
    let st = state();
    let mut r = req("POST", "/orders.json");
    r.set_test_header("content-type", "application/x-www-form-urlencoded");
    r.set_test_header("origin", "https://kiosk.bixbyapps.com");
    r.set_test_body(b"total=3&items[]=a&items[]=b".to_vec());
    assert_eq!(handle(&st, r).status, 201);
    let rows = st.pool.with(|db| db.query("SELECT total, items FROM orders", &[])).expect("query");
    assert_eq!(rows[0].get("total"), Some(&crate::db::Value::Int(3)));
    assert_eq!(rows[0].text("items"), Some(r#"["a","b"]"#));
}

#[test]
fn post_orders_bad_json_is_400() {
    let res = handle(&state(), post_json("/orders", "{bad", Some("https://kiosk.bixbyapps.com")));
    assert_eq!((res.status, body(&res).as_str()), (400, "bad request page"));
}

const SAME_ORIGIN: &str = "https://kiosk.bixbyapps.com";

/// A form `POST /orders` with `n` nested `[a]` segments under `items`.
fn post_nested_form(n: usize, origin: Option<&str>) -> Request {
    let mut r = req("POST", "/orders");
    r.set_test_header("content-type", "application/x-www-form-urlencoded");
    if let Some(o) = origin {
        r.set_test_header("origin", o);
    }
    r.set_test_body(format!("items{}=1&total=1", "[a]".repeat(n)).into_bytes());
    r
}

fn order_count(st: &AppState) -> usize {
    st.pool.with(|db| db.query("SELECT id FROM orders", &[])).expect("query").len()
}

#[test]
fn post_orders_bad_json_without_origin_is_400_like_rails() {
    // Rails reads the JSON body (400) before failing the token check (422).
    let res = handle(&state(), post_json("/orders", "{bad", None));
    assert_eq!(res.status, 400);
}

#[test]
fn post_orders_bad_json_cross_origin_is_422_like_rails() {
    // Rails rejects a foreign Origin before reading the body.
    let res = handle(&state(), post_json("/orders", "{bad", Some("https://evil.example")));
    assert_eq!(res.status, 422);
}

#[test]
fn post_orders_deep_form_is_400_and_server_keeps_serving() {
    // QA's attack shape, cut to fit the 16 KiB cap: 2,000 levels, same origin.
    let st = state();
    let mut r = post_nested_form(2_000, None);
    r.set_test_header("sec-fetch-site", "same-origin");
    let res = handle(&st, r);
    assert_eq!((res.status, body(&res).as_str()), (400, "bad request page"));
    assert_eq!(order_count(&st), 0);
    assert_eq!(handle(&st, req("GET", "/menu_items")).status, 200);
}

#[test]
fn post_orders_depth_33_form_is_400() {
    assert_eq!(handle(&state(), post_nested_form(32, Some(SAME_ORIGIN))).status, 400);
}

#[test]
fn post_orders_depth_32_form_saves() {
    let st = state();
    assert_eq!(handle(&st, post_nested_form(31, Some(SAME_ORIGIN))).status, 201);
    assert_eq!(order_count(&st), 1);
}

#[test]
fn post_orders_deep_form_is_400_before_browser_and_origin_checks() {
    // Rack::MethodOverride parses the form first: Rails answered 400 for an
    // unsupported browser and for a foreign Origin alike.
    let mut old = post_nested_form(40, Some("https://evil.example"));
    old.set_test_header("user-agent", CHROME_119);
    assert_eq!(handle(&state(), old).status, 400);
}

#[test]
fn post_orders_over_16_kib_is_413() {
    let st = state();
    let mut r = post_json("/orders", &format!(r#"{{"items":"{}"}}"#, "x".repeat(ORDER_BODY_MAX)), Some(SAME_ORIGIN));
    r.set_test_header("sec-fetch-site", "same-origin");
    assert_eq!(handle(&st, r).status, 413);
    assert_eq!(order_count(&st), 0);
}

#[test]
fn post_orders_qa_60kb_payload_is_413() {
    assert_eq!(handle(&state(), post_nested_form(20_000, Some(SAME_ORIGIN))).status, 413);
}

#[test]
fn post_orders_at_16_kib_is_not_413() {
    let pad = ORDER_BODY_MAX - r#"{"items":""}"#.len();
    let r = post_json("/orders", &format!(r#"{{"items":"{}"}}"#, "x".repeat(pad)), Some(SAME_ORIGIN));
    assert_eq!(handle(&state(), r).status, 201);
}

#[test]
fn post_orders_eleventh_in_a_minute_is_429() {
    let st = state();
    let ip = format!("rate-{}", SEQ.fetch_add(1, Ordering::Relaxed));
    let post = || {
        let mut r = post_json("/orders", r#"{"total":"1"}"#, Some(SAME_ORIGIN));
        r.peer_ip = ip.clone();
        r
    };
    for _ in 0..limit::ORDER_LIMIT {
        assert_eq!(handle(&st, post()).status, 201);
    }
    let res = handle(&st, post());
    assert_eq!(res.status, 429);
    assert!(header(&res, "retry-after").is_some_and(|v| v.parse::<i64>().is_ok_and(|s| (1..=60).contains(&s))));
    assert_eq!(order_count(&st), limit::ORDER_LIMIT);
}

#[test]
fn post_orders_limit_counts_rejected_posts() {
    let st = state();
    let ip = format!("rate-{}", SEQ.fetch_add(1, Ordering::Relaxed));
    for _ in 0..limit::ORDER_LIMIT {
        let mut r = post_json("/orders", "{}", Some("https://evil.example"));
        r.peer_ip = ip.clone();
        assert_eq!(handle(&st, r).status, 422);
    }
    let mut r = post_json("/orders", "{}", Some(SAME_ORIGIN));
    r.peer_ip = ip;
    assert_eq!(handle(&st, r).status, 429);
}

#[test]
fn get_orders_is_404() {
    assert_eq!(handle(&state(), req("GET", "/orders")).status, 404);
}

#[test]
fn post_menu_items_is_404() {
    assert_eq!(handle(&state(), req("POST", "/menu_items")).status, 404);
}

#[test]
fn csp_allows_data_images_and_no_third_party_hosts() {
    let res = handle(&state(), req("GET", "/"));
    let csp = header(&res, "content-security-policy").expect("csp");
    assert!(csp.contains("script-src 'self';") && csp.contains("connect-src 'self';") && csp.contains("img-src 'self' https: data:"));
}
