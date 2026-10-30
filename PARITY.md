# Kiosk parity checklist

Source: `~/Projects/kiosk` (Rails 8.0.2, commit `7c8be77`, version 0.4.0), read-only.
Live: `https://kiosk.bixbyapps.com` (Rails + Thruster in a Docker image, port 80, no volume).
Live GETs on 2026-10-30 matched the repo (only the CDN's injected challenge script differs).

Legend: `[ ]` to do, `[x]` built + tested + verified, `[-]` not ported on purpose (reason given).

## 1. Routes (config/routes.rb + framework)

- [ ] `GET /` → `PagesController#index`: renders `public/kiosk.html` with `layout: false` (200, `text/html; charset=utf-8`)
- [ ] `GET /menu_items` → `MenuItemsController#index`: `render json: MenuItem.all` (200, `application/json; charset=utf-8`)
- [ ] `GET /menu_items.json` → same as above (Rails format suffix)
- [ ] `POST /orders` → `OrdersController#create`: `Order.create!(total:, items:)`, 201 `{"message":"Order created successfully","order_id":<id>}`, any error 422 `{"error":"<message>"}`
- [ ] `POST /orders` CSRF: `ActionController::Base` default `protect_from_forgery` rejects a POST without a valid token (422 HTML). `kiosk.html` is rendered without the layout, so no token is ever issued: in production every POST /orders fails. Confirm against a local Rails copy
- [ ] `/up` health: NOT routed in Rails (commented out). Live `GET /up` → 404 HTML. `production.rb` still names `/up` in `silence_healthcheck_path`. Rust app keeps `/up` → 404 parity and adds skateboard `GET /api/health` for the container health check
- [ ] Unknown path → 404 with `public/404.html` body (`text/html; charset=UTF-8`)
- [-] `/cable` (Action Cable default mount, live 404 text/plain): no channels, no consumer JS. Not ported
- [-] `/rails/active_storage/*`, `/rails/action_mailbox/*`, `/rails/conductor/*` (framework routes from `rails/all`): no Active Storage tables, no attachments, no mailboxes. Live returns 404. Not ported
- [-] `GET /manifest`, `GET /service-worker` (PWA): commented out in routes, live 404. Not ported (stays 404)

## 2. Static files (public/, served by Rails/Thruster)

- [ ] `/kiosk.html` (same page as `/`)
- [ ] `/icon.png` (512x512 PNG), `/icon.svg` (red circle)
- [ ] `/images/frankie-logo.png` (560x177 PNG)
- [ ] `/robots.txt` (comment only, allow all)
- [ ] `/400.html`, `/404.html`, `/406-unsupported-browser.html`, `/422.html`, `/500.html` (Rails error pages, also served as files)

## 3. Models, schema, migrations

- [ ] `menu_items`: `id` INTEGER PK, `name` varchar, `category` varchar, `created_at` datetime NOT NULL, `updated_at` datetime NOT NULL, `price` decimal
- [ ] `orders`: `id` INTEGER PK, `total` decimal, `status` varchar, `created_at`, `updated_at` NOT NULL, `items` text (JSON-serialized, `serialize :items, coder: JSON`)
- [ ] `order_items`: `id` PK, `order_id` NOT NULL FK → orders, `menu_item_id` NOT NULL FK → menu_items, `quantity` integer, timestamps, indexes on both FKs. Unused by app code (no writes, no reads). Ported as schema only
- [ ] Migrations 20250718013132, 013211, 013247, 013350, 020859: final schema = `db/schema.rb` version 2025_07_18_020859
- [ ] `OrderItem belongs_to :order, :menu_item` (schema FKs only; no code path)

## 4. Seeds and boot

- [ ] `db/seeds.rb`: 19 menu items (name + price, category NULL), ids 1..19 in file order
- [ ] `bin/docker-entrypoint` runs `db:prepare` on server start: creates the DB, loads schema, seeds only when the DB is created. Live has no volume, so every deploy re-seeds
- [ ] Rust: create schema if missing, seed only on first creation, DB file on a mounted volume path

## 5. JSON shape (exact)

- [ ] Menu item keys in column order: `id`, `name`, `category`, `created_at`, `updated_at`, `price`
- [ ] `price` is a JSON string (BigDecimal): `"1.5"`, `"1.99"`, `"0.79"`
- [ ] Timestamps ISO 8601 UTC with milliseconds: `"2026-10-28T01:10:18.559Z"`
- [ ] Rails JSON escapes `&` `<` `>` as `&` `<` `>` (live: `"Hotdog & Soda"`)
- [ ] Array in id order

## 6. Front end (public/kiosk.html)

- [ ] Title "Frankie Food Court Kiosk", logo, heading "Welcome to the Frankie Food Court Kiosk!", tap hint
- [ ] Menu grid (8 columns) from `GET /menu_items`: image placeholder, name, `$<price>` as served (e.g. `$1.5`)
- [ ] Tap item → add to cart (merge by id, qty +1)
- [ ] Cart list: name, "Quantity N", Edit button, line total `$x.xx`; tapping a row removes it
- [ ] Edit modal "Quantity Change": minus (floor 1), plus, Remove Item, Cancel, OK
- [ ] Sub-Total, Tax (8%), Total (x1.08); tax/total hidden (opacity-0) while cart has items, shown when empty
- [ ] Cart hint "You can change or remove an item on the receipt by tapping it." when cart not empty
- [ ] CANCEL ORDER clears cart; PLACE ORDER → checkout screen
- [ ] Checkout screen: "Balance Due: $<total x1.08>", "Follow prompts on PIN pad.", PIN pad image, "Return to Menu"
- [ ] Cart + total persisted in `localStorage` (`cart`, `total`), restored on load
- [ ] Empty-cart PLACE ORDER guard (`alert`, unreachable because the button hides when empty)
- [ ] Note: the page never calls `POST /orders` (checkout is a mock PIN pad)

## 7. Analytics

- [ ] Third-party analytics tag on the kiosk page (public site id, not a secret)
- [-] Same tag in `layouts/application.html.erb`: the layout is never rendered (only route uses `layout: false`). Covered by the page tag above

## 8. PWA

- [-] `app/views/pwa/manifest.json.erb` + `service-worker.js` (all commented): not routed, not linked. Not ported (live 404)
- [ ] Icons `/icon.png`, `/icon.svg` (see static files)

## 9. Background jobs, mailers, storage, auth, admin

- [-] Solid Queue (`SOLID_QUEUE_IN_PUMA`, `config/queue.yml`, `recurring.yml` hourly `clear_finished_in_batches`): only housekeeping for its own queue tables, no app jobs. Not ported
- [-] Solid Cache, Solid Cable DBs (`production_cache/queue/cable.sqlite3`): framework stores, no app use. Not ported
- [-] `ApplicationMailer` (from@example.com), no mailers, no SMTP. Not ported
- [-] Active Storage (`config/storage.yml` local Disk): no attachments, no tables. Not ported
- [ ] Auth: none. Admin: none. Rust app runs skateboard with `noLogin: true` (no login UI)
- [ ] `allow_browser versions: :modern`: old browsers get 406 + `406-unsupported-browser.html`. Decide (see REPORT)
- [-] Stimulus `hello_controller.js`, Turbo, importmap: loaded only by the unused layout. Not ported

## 10. Config and env var NAMES (no values)

- [ ] Rails: `RAILS_MASTER_KEY`, `SECRET_KEY_BASE` (via credentials), `RAILS_ENV`, `RAILS_MAX_THREADS`, `RAILS_LOG_LEVEL`, `PORT`, `PIDFILE`, `WEB_CONCURRENCY`, `JOB_CONCURRENCY`, `SOLID_QUEUE_IN_PUMA`, `LD_PRELOAD`, Thruster `HTTP_PORT`/`TARGET_PORT` (defaults), Kamal `KAMAL_REGISTRY_PASSWORD`
- [ ] Rust: list its own env var names in README (skateboard: `NODE_ENV`, `PORT`, `JWT_SECRET`, `CORS_ORIGINS`, ...)
- [-] Kamal `config/deploy.yml`, `.kamal/*`: unused in production (the app is deployed as a plain Docker image). Not ported
- [ ] Production headers: `force_ssl` + `assume_ssl` (HSTS behind the CDN), `X-Frame-Options: SAMEORIGIN`, `X-Content-Type-Options: nosniff`, `Referrer-Policy: strict-origin-when-cross-origin`

## 11. Tests in the Rails repo

- [-] `test/models/*_test.rb`: empty (commented "the truth"). Fixtures only. Nothing to port; Rust and front-end tests are new

## 12. Deploy shape

- [ ] Dockerfile: skateboard style, base images pinned by digest, `cargo build --release --locked`
- [ ] SQLite on a mounted volume path, not in the image
- [ ] README: SQLite backup notes (describe only)
