//! Kiosk routes: the Rails app's `config/routes.rb`, plus its `public/` files.
//!
//! | Rails | Here |
//! |---|---|
//! | `root 'pages#index'` | `GET /` → the kiosk page (`dist/index.html`) |
//! | `resources :menu_items, only: [:index]` | `GET /menu_items(.:format)` |
//! | `resources :orders, only: [:create]` | `POST /orders(.:format)` |
//! | `public/` (ActionDispatch::Static) | files in `dist/`, `path` then `path.html` |
//!
//! Everything else answers 404 with Rails' `404.html`, including the skateboard
//! template's account and billing API: the kiosk has no login. Only
//! `GET /api/health` (container health check) and CORS preflights pass through
//! to the template router.

pub mod browser;
pub mod encode;
pub mod limit;
pub mod params;
pub mod store;

use crate::config;
use crate::crypto;
use crate::http::{Request, Response};
use crate::middleware;
use crate::routes;
use crate::state::AppState;
use limit::OrderLimiter;
use params::Param;
use std::sync::LazyLock;

/// The template CSP plus `data:` images (the checkout screen's inline PIN pad
/// SVG). No third-party script or beacon hosts.
const CSP: &str = "default-src 'self'; \
script-src 'self'; \
style-src 'self' 'unsafe-inline'; \
img-src 'self' https: data:; \
font-src 'self'; \
connect-src 'self'; \
frame-ancestors 'none'";

/// Rails' `Cache-Control` for rendered actions.
const DYNAMIC_CACHE: &str = "max-age=0, private, must-revalidate";

/// `config.public_file_server.headers` in `production.rb` (`1.year`).
const STATIC_CACHE: &str = "public, max-age=31556952";

/// Largest `POST /orders` body (16 KiB); larger gets 413. A real order is a
/// few hundred bytes. The server-wide 2 MiB read cap still applies first.
pub const ORDER_BODY_MAX: usize = 16 * 1024;

/// Process-wide `POST /orders` rate limit (10 per minute per client IP).
static ORDER_LIMITER: LazyLock<OrderLimiter> = LazyLock::new(OrderLimiter::new);

/// Create the kiosk tables and seed the menu on a new database.
///
/// # Errors
/// A message naming the SQLite failure; `main` exits on it.
pub fn prepare(state: &AppState) -> Result<(), String> {
    let seeded = store::prepare(&state.pool).map_err(|e| format!("kiosk schema: {e}"))?;
    if seeded {
        state.log.info(
            "Seeded kiosk menu",
            &[("items", crate::json::i(store::SEED_MENU.len() as i64))],
        );
    }
    Ok(())
}

/// Entry point for every request.
pub fn handle(state: &AppState, req: Request) -> Response {
    if req.method.eq_ignore_ascii_case("OPTIONS") || req.path == "/api/health" {
        return routes::handle(state, req);
    }
    let start = config::now_ms();
    middleware::dev_request_log(&state.log, &req, state.prod);
    let res = dispatch(state, &req);
    let res = middleware::apply_secure_headers(res, state.prod)
        .set_header("Content-Security-Policy", CSP);
    middleware::access_log(&req.method, &req.path, res.status, config::now_ms() - start);
    res
}

/// A matched Rails route.
#[derive(Debug, PartialEq, Eq)]
enum Route {
    Root,
    MenuItems,
    Orders,
    Other,
}

/// Collapse repeated slashes and drop a trailing one, as Rails' router does.
fn normalize(raw: &str) -> String {
    let joined: Vec<&str> = raw.split('/').filter(|s| !s.is_empty()).collect();
    format!("/{}", joined.join("/"))
}

/// Match `path` (already normalized) to a route, honoring `(.:format)`.
fn route(path: &str) -> Route {
    if path == "/" {
        return Route::Root;
    }
    let with_format = |name: &str| {
        path == name
            || path
                .strip_prefix(name)
                .and_then(|rest| rest.strip_prefix('.'))
                .is_some_and(|fmt| !fmt.is_empty() && !fmt.contains(['.', '/']))
    };
    if with_format("/menu_items") {
        Route::MenuItems
    } else if with_format("/orders") {
        Route::Orders
    } else {
        Route::Other
    }
}

fn dispatch(state: &AppState, req: &Request) -> Response {
    let path = normalize(&req.raw_path);
    let is_read = req.method == "GET" || req.method == "HEAD";
    match (route(&path), is_read, req.method == "POST") {
        (Route::Root, true, _) => controller(state, req, || page(state, req)),
        (Route::MenuItems, true, _) => controller(state, req, || menu(state, req)),
        (Route::Orders, _, true) => post_orders(state, req),
        (Route::Other, true, _) if !path.starts_with("/api/") => static_file(state, &path),
        _ => error_page(state, 404),
    }
}

/// Run a controller action behind `allow_browser versions: :modern`.
fn controller(state: &AppState, req: &Request, action: impl FnOnce() -> Response) -> Response {
    if browser::is_blocked(req.header("user-agent")) {
        return error_page(state, 406);
    }
    action()
}

/// `PagesController#index`: the kiosk page, uncached, with an ETag.
fn page(state: &AppState, req: &Request) -> Response {
    match read_dist(state, "index.html") {
        Some(body) => conditional(req, "text/html; charset=utf-8", body),
        None => error_page(state, 500),
    }
}

/// `MenuItemsController#index`: `render json: MenuItem.all`.
fn menu(state: &AppState, req: &Request) -> Response {
    match store::menu_json(&state.pool) {
        Ok(json) => conditional(req, "application/json; charset=utf-8", json.into_bytes()),
        Err(e) => {
            state.log.error(
                "Menu query failed",
                &[("error", crate::json::s(e.to_string()))],
            );
            error_page(state, 500)
        }
    }
}

/// `POST /orders`: the guards in front of [`create_order`].
///
/// Not in Rails: the 16 KiB body cap (413) and the per-IP limit (429). Then
/// Rails' order: Rack::MethodOverride parses a form body before the
/// controller, so a form past Rack's limits is 400 even for an old browser
/// (406) or a foreign origin (422).
fn post_orders(state: &AppState, req: &Request) -> Response {
    if req.body.len() > ORDER_BODY_MAX {
        return Response::text(413, "Payload Too Large");
    }
    if let Err(secs) = ORDER_LIMITER.check(&routes::client_ip(req), config::now_ms()) {
        return Response::text(429, "Too Many Requests")
            .set_header("Retry-After", &secs.to_string());
    }
    let form = match form_body(req) {
        Ok(form) => form,
        Err(_) => return error_page(state, 400),
    };
    controller(state, req, || create_order(state, req, form))
}

/// `OrdersController#create`.
///
/// Rails guards this with `protect_from_forgery`, and the kiosk page never
/// receives a CSRF token, so in Rails every POST fails with 422. Here a
/// same-origin browser request (matching `Origin`, or `Sec-Fetch-Site:
/// same-origin` when no `Origin` is sent) stands in for the token check and
/// is saved (Steve, 2026-10-30; REPORT.md decision 1). Anything else gets
/// Rails' 422 page, in Rails' order: a foreign `Origin` before the JSON body
/// is read, a missing token after it (bad JSON is then 400).
///
/// `form` is the already-parsed form body ([`form_body`]).
fn create_order(state: &AppState, req: &Request, form: Option<Param>) -> Response {
    if req.header("origin").is_some() && !is_same_origin(req) {
        return error_page(state, 422);
    }
    let body = match form.map_or_else(|| json_body(req), Ok) {
        Ok(p) => p,
        Err(params::InvalidJson) => return error_page(state, 400),
    };
    if !is_same_origin(req) {
        return error_page(state, 422);
    }
    let Ok(query) = params::parse_form(&req.query) else {
        return error_page(state, 400);
    };
    let params = params::merge(body, query);
    let total = params::cast_decimal(params.get("total"));
    let items = params
        .get("items")
        .filter(|v| **v != Param::Null)
        .map(params::encode_items);
    match store::create_order(&state.pool, total, items) {
        Ok(id) => Response::bytes(
            201,
            "application/json; charset=utf-8",
            format!(r#"{{"message":"Order created successfully","order_id":{id}}}"#).into_bytes(),
        )
        .set_header("Cache-Control", DYNAMIC_CACHE),
        Err(e) => {
            let mut out = String::from(r#"{"error":"#);
            encode::push_str(&mut out, &e.message);
            out.push('}');
            Response::bytes(422, "application/json; charset=utf-8", out.into_bytes())
        }
    }
}

/// A non-blank form-encoded body, parsed; `Ok(None)` for any other body.
///
/// # Errors
/// [`params::FormLimit`] past Rack's depth (32) or pair (4096) limit.
fn form_body(req: &Request) -> Result<Option<Param>, params::FormLimit> {
    let ct = req
        .header("content-type")
        .unwrap_or("")
        .to_ascii_lowercase();
    if !ct.starts_with("application/x-www-form-urlencoded") || is_blank(&req.body) {
        return Ok(None);
    }
    params::parse_form(&String::from_utf8_lossy(&req.body)).map(Some)
}

/// Body params for a JSON post; any other body contributes none.
fn json_body(req: &Request) -> Result<Param, params::InvalidJson> {
    let ct = req
        .header("content-type")
        .unwrap_or("")
        .to_ascii_lowercase();
    if !ct.starts_with("application/json") || is_blank(&req.body) {
        return Ok(Param::Obj(Vec::new()));
    }
    Ok(match params::parse_json(&req.body)? {
        obj @ Param::Obj(_) => obj,
        _ => Param::Obj(Vec::new()),
    })
}

fn is_blank(body: &[u8]) -> bool {
    body.iter().all(u8::is_ascii_whitespace)
}

/// Same-origin check that stands in for Rails' CSRF token (see [`create_order`]).
fn is_same_origin(req: &Request) -> bool {
    let Some(host) = req.header("host").filter(|h| !h.is_empty()) else {
        return false;
    };
    match req.header("origin") {
        Some(origin) => origin == format!("https://{host}") || origin == format!("http://{host}"),
        None => req.header("sec-fetch-site") == Some("same-origin"),
    }
}

/// 200 with a weak ETag, or 304 when `If-None-Match` already has it
/// (Rack::ETag + Rack::ConditionalGet in the Rails stack).
fn conditional(req: &Request, content_type: &str, body: Vec<u8>) -> Response {
    let digest = crypto::hex_encode(&crypto::sha256(&body));
    let etag = format!("W/\"{}\"", &digest[..32]);
    let matched = req
        .header("if-none-match")
        .is_some_and(|v| v.split(',').any(|t| t.trim() == etag || t.trim() == "*"));
    let res = if matched {
        Response::empty(304)
    } else {
        Response::bytes(200, content_type, body)
    };
    res.set_header("ETag", &etag)
        .set_header("Cache-Control", DYNAMIC_CACHE)
}

/// Serve a public file: `path`, then `path.html` (ActionDispatch::Static).
///
/// `/kiosk.html` (and `/kiosk`) is the same page as `/`, as it was in Rails'
/// `public/`. The Vite entry `index.html` itself is not a public URL.
fn static_file(state: &AppState, path: &str) -> Response {
    match path {
        "/kiosk.html" | "/kiosk" => {
            return match read_dist(state, "index.html") {
                Some(body) => Response::bytes(200, "text/html; charset=utf-8", body)
                    .set_header("Cache-Control", DYNAMIC_CACHE),
                None => error_page(state, 404),
            };
        }
        "/index.html" | "/index" => return error_page(state, 404),
        _ => {}
    }
    crate::http::serve_file(&state.static_dir, path)
        .or_else(|| crate::http::serve_file(&state.static_dir, &format!("{path}.html")))
        .map_or_else(
            || error_page(state, 404),
            |res| res.set_header("Cache-Control", STATIC_CACHE),
        )
}

/// Read a file from the built frontend directory.
fn read_dist(state: &AppState, name: &str) -> Option<Vec<u8>> {
    std::fs::read(state.static_dir.join(name)).ok()
}

/// Rails' static error page for `status` (`public/<status>.html`).
fn error_page(state: &AppState, status: u16) -> Response {
    let file = if status == 406 {
        "406-unsupported-browser.html".to_string()
    } else {
        format!("{status}.html")
    };
    // `render file:` (406) answers lowercase utf-8; the exceptions app, uppercase.
    let content_type = if status == 406 {
        "text/html; charset=utf-8"
    } else {
        "text/html; charset=UTF-8"
    };
    match read_dist(state, &file) {
        Some(body) => Response::bytes(status, content_type, body),
        None => Response::text(status, &format!("{status}")),
    }
}

#[cfg(test)]
mod tests;
