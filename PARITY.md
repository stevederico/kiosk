# Kiosk parity checklist

Source: `~/Projects/kiosk` (Rails 8.0.2, commit `7c8be77`, version 0.4.0), read-only.
Target: this repo on skateboard 5.13.0 / skateboard-ui 5.11.0.
Live: `https://kiosk.bixbyapps.com` (Rails + Thruster in a Docker image, port 80, no volume).
Live GETs on 2026-10-30 matched the repo (only the CDN's injected challenge script differs).

Legend: `[ ]` to do, `[x]` built + tested + verified, `[-]` not ported on purpose (reason given).

## 1. Routes (config/routes.rb + framework)

- [x] `GET /` → `PagesController#index`: renders `public/kiosk.html` with `layout: false` (200, `text/html; charset=utf-8`)
- [x] `GET /menu_items` → `MenuItemsController#index`: `render json: MenuItem.all` (200, `application/json; charset=utf-8`)
- [x] `GET /menu_items.json` → same as above (Rails format suffix)
- [x] `POST /orders` → `OrdersController#create`: `Order.create!(total:, items:)`, 201 `{"message":"Order created successfully","order_id":<id>}`, any error 422 `{"error":"<message>"}`
- [x] `POST /orders` CSRF: `ActionController::Base` default `protect_from_forgery` rejects a POST without a valid token (422 HTML). `kiosk.html` is rendered without the layout, so no token is ever issued: in production every POST /orders fails. Confirm against a local Rails copy
- [x] `POST /orders` param limits (captured 2026-10-30 from the Rails copy): Rack rejects a form key nested past 32 levels (the name counts; a final `[]` does not) or more than 4096 pairs with 400, and does so in `Rack::MethodOverride`, before `allow_browser` (406) and the CSRF check (422). A foreign `Origin` is 422 before the JSON body is read; with no `Origin`, bad JSON is 400. A query string past the limits is ignored (422). The Rust app matches all of these
- [x] `/up` health: NOT routed in Rails (commented out). Live `GET /up` → 404 HTML. `production.rb` still names `/up` in `silence_healthcheck_path`. Rust app keeps `/up` → 404 parity and adds skateboard `GET /api/health` for the container health check
- [x] Unknown path → 404 with `public/404.html` body (`text/html; charset=UTF-8`)
- [-] `/cable` (Action Cable default mount, live 404 text/plain): no channels, no consumer JS. Not ported
- [-] `/rails/active_storage/*`, `/rails/action_mailbox/*`, `/rails/conductor/*` (framework routes from `rails/all`): no Active Storage tables, no attachments, no mailboxes. Live returns 404. Not ported
- [-] `GET /manifest`, `GET /service-worker` (PWA): commented out in routes, live 404. Not ported (stays 404)

## 2. Static files (public/, served by Rails/Thruster)

- [x] `/kiosk.html` (same page as `/`)
- [x] `/icon.png` (512x512 PNG), `/icon.svg` (red circle)
- [x] `/images/frankie-logo.png` (560x177 PNG)
- [x] `/robots.txt` (comment only, allow all)
- [x] `/400.html`, `/404.html`, `/406-unsupported-browser.html`, `/422.html`, `/500.html` (Rails error pages, also served as files)

## 3. Models, schema, migrations

- [x] `menu_items`: `id` INTEGER PK, `name` varchar, `category` varchar, `created_at` datetime NOT NULL, `updated_at` datetime NOT NULL, `price` decimal
- [x] `orders`: `id` INTEGER PK, `total` decimal, `status` varchar, `created_at`, `updated_at` NOT NULL, `items` text (JSON-serialized, `serialize :items, coder: JSON`)
- [x] `order_items`: `id` PK, `order_id` NOT NULL FK → orders, `menu_item_id` NOT NULL FK → menu_items, `quantity` integer, timestamps, indexes on both FKs. Unused by app code (no writes, no reads). Ported as schema only
- [x] Migrations 20250718013132, 013211, 013247, 013350, 020859: final schema = `db/schema.rb` version 2025_07_18_020859
- [x] `OrderItem belongs_to :order, :menu_item` (schema FKs only; no code path)

## 4. Seeds and boot

- [x] `db/seeds.rb`: 19 menu items (name + price, category NULL), ids 1..19 in file order
- [x] `bin/docker-entrypoint` runs `db:prepare` on server start: creates the DB, loads schema, seeds only when the DB is created. Live has no volume, so every deploy re-seeds
- [x] Rust: create schema if missing, seed only on first creation, DB file on a mounted volume path

## 5. JSON shape (exact)

- [x] Menu item keys in column order: `id`, `name`, `category`, `created_at`, `updated_at`, `price`
- [x] `price` is a JSON string (BigDecimal): `"1.5"`, `"1.99"`, `"0.79"`
- [x] Timestamps ISO 8601 UTC with milliseconds: `"2026-10-28T01:10:18.559Z"`
- [x] Rails JSON escapes `&` `<` `>` as `&` `<` `>` (live: `"Hotdog & Soda"`)
- [x] Array in id order

## 6. Front end (public/kiosk.html)

- [x] Title "Frankie Food Court Kiosk", logo, heading "Welcome to the Frankie Food Court Kiosk!", tap hint
- [x] Menu grid (8 columns) from `GET /menu_items`: image placeholder, name, `$<price>` as served (e.g. `$1.5`)
- [x] Tap item → add to cart (merge by id, qty +1)
- [x] Cart list: name, "Quantity N", Edit button, line total `$x.xx`; tapping a row removes it
- [x] Edit modal "Quantity Change": minus (floor 1), plus, Remove Item, Cancel, OK
- [x] Sub-Total, Tax (8%), Total (x1.08); tax/total hidden (opacity-0) while cart has items, shown when empty
- [x] Cart hint "You can change or remove an item on the receipt by tapping it." when cart not empty
- [x] CANCEL ORDER clears cart; PLACE ORDER → checkout screen
- [x] Checkout screen: "Balance Due: $<total x1.08>", "Follow prompts on PIN pad.", PIN pad image, "Return to Menu"
- [x] Cart + total persisted in `localStorage` (`cart`, `total`), restored on load
- [x] Empty-cart PLACE ORDER guard (`alert`, unreachable because the button hides when empty)
- [x] Note: the page never calls `POST /orders` (checkout is a mock PIN pad)

## 7. Analytics

- [x] Third-party analytics tag on the kiosk page (public site id, not a secret)
- [-] Same tag in `layouts/application.html.erb`: the layout is never rendered (only route uses `layout: false`). Covered by the page tag above

## 8. PWA

- [-] `app/views/pwa/manifest.json.erb` + `service-worker.js` (all commented): not routed, not linked. Not ported (live 404)
- [x] Icons `/icon.png`, `/icon.svg` (see static files)

## 9. Background jobs, mailers, storage, auth, admin

- [-] Solid Queue (`SOLID_QUEUE_IN_PUMA`, `config/queue.yml`, `recurring.yml` hourly `clear_finished_in_batches`): only housekeeping for its own queue tables, no app jobs. Not ported
- [-] Solid Cache, Solid Cable DBs (`production_cache/queue/cable.sqlite3`): framework stores, no app use. Not ported
- [-] `ApplicationMailer` (from@example.com), no mailers, no SMTP. Not ported
- [-] Active Storage (`config/storage.yml` local Disk): no attachments, no tables. Not ported
- [x] Auth: none. Admin: none. Rust app runs skateboard with `noLogin: true` (no login UI)
- [x] `allow_browser versions: :modern`: old browsers get 406 + `406-unsupported-browser.html`. Decide (see REPORT)
- [-] Stimulus `hello_controller.js`, Turbo, importmap: loaded only by the unused layout. Not ported

## 10. Config and env var NAMES (no values)

- [x] Rails: `RAILS_MASTER_KEY`, `SECRET_KEY_BASE` (via credentials), `RAILS_ENV`, `RAILS_MAX_THREADS`, `RAILS_LOG_LEVEL`, `PORT`, `PIDFILE`, `WEB_CONCURRENCY`, `JOB_CONCURRENCY`, `SOLID_QUEUE_IN_PUMA`, `LD_PRELOAD`, Thruster `HTTP_PORT`/`TARGET_PORT` (defaults), Kamal `KAMAL_REGISTRY_PASSWORD`
- [x] Rust: list its own env var names in README (skateboard: `NODE_ENV`, `PORT`, `JWT_SECRET`, `CORS_ORIGINS`, ...)
- [-] Kamal `config/deploy.yml`, `.kamal/*`: unused in production (the app is deployed as a plain Docker image). Not ported
- [x] Production headers: `force_ssl` + `assume_ssl` (HSTS behind the CDN), `X-Frame-Options: SAMEORIGIN`, `X-Content-Type-Options: nosniff`, `Referrer-Policy: strict-origin-when-cross-origin`

## 11. Tests in the Rails repo

- [-] `test/models/*_test.rb`: empty (commented "the truth"). Fixtures only. Nothing to port; Rust and front-end tests are new

## 12. Deploy shape

- [x] Dockerfile: skateboard style, base images pinned by digest, `cargo build --release --locked`
- [x] SQLite on a mounted volume path, not in the image
- [x] README: SQLite backup notes (describe only)

## Results (2026-10-30)

How each item was checked:

- HTTP: `scripts/parity/http-parity.sh` ran 50 requests against the Rails app (built from a copy at `~/Projects/kiosk-rails-compare`, port 38080) and this app's Docker image (port 38200). 49 of 50 match. JSON routes, 406/422/404 responses: status, content type and body bytes. Static files and error pages: status and body bytes (content type differs only by a `; charset=UTF-8` suffix on `.txt`/`.html`). `/`, `/kiosk.html`: status and content type (the HTML body is the React build, by design). `/menu_items` bodies are byte-identical after masking the seed timestamps. `/cable`: 404 on both (Rails body is plain text, here the 404 page). Only `GET /api/health` differs (new, see below)
- Behavior: 58 Rust tests in `backend/src/kiosk/` use values captured from the Rails app (UA matrix of 32 agents, decimal casts, float and string encodings, stored order rows)
- Page: the same scripted browser run (`scripts/parity/ui-scenario.sh`) (add, edit, remove by row, remove in dialog, place order, return, reload, cancel) gave identical totals, visibility and `localStorage` on both apps; screenshots of the menu, dialog and checkout match
- Data: order created, container restarted, menu not re-seeded, orders kept, `PRAGMA integrity_check` ok on the volume copy

Re-checked after the update to skateboard 5.13.0 / skateboard-ui 5.11.0 (`cfec801`): same results. 49 of 50 HTTP checks match (only `/api/health`), the scripted page run matches, data survives a restart. Details in REPORT.md, "Upgrade to 5.13.0"

Re-checked for the QA gate fixes (0.7.0, `kiosk-gatefix:0.7.0` vs the Rails copy, both local): 49 of 50 HTTP checks match (only `/api/health`). Ten extra `POST /orders` probes (form depth 32 / 33 / 41 / 2001, foreign Origin, old Chrome, bad JSON with and without Origin) match Rails status for status, except the three deliberate ones: QA's 60 KB payload is 413 (Rails 400), a 17 KB body is 413 (Rails 422), a spoofed same-origin JSON order is 201 (Rails 422)

Differences on purpose (details and Steve's decisions in REPORT.md):

- `POST /orders`: Rails rejects every request (CSRF, no token is ever issued). Here a same-origin browser request (`Origin` = host, or `Sec-Fetch-Site: same-origin`) is saved, a deliberate difference Steve approved on 2026-10-30 (REPORT.md decision 1); everything else gets the same 422 page. Not in Rails either: a body over 16 KiB is 413 and an 11th POST in a minute from one IP is 429
- `GET /api/health`: added for the container health check (Rails: 404)
- Headers: skateboard's security headers (`X-Frame-Options: DENY`, CSP with the analytics host, HSTS 1 year + preload) instead of Rails' (`SAMEORIGIN`, no CSP, HSTS 2 years). `Cache-Control` and weak `ETag`/304 match Rails' shape; ETag values differ (sha256 vs md5)
- `/kiosk.html` is not cached for a year (Rails: `public, max-age=31556952`), so a deploy is never hidden behind a stale page
- Page: emoji icons (pointer, hot dog) are Lucide icons; the edit dialog closes on Escape and traps focus; a failed menu load shows an error with Try Again (Rails showed an empty grid); the unreachable "Your cart is empty!" `alert()` is dropped
- The menu grid unmounts on checkout instead of staying hidden in the DOM (same on screen)
