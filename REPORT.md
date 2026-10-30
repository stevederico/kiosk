# Kiosk rewrite report

Rails 8 kiosk (`~/Projects/kiosk` @ `7c8be77`, live at kiosk.bixbyapps.com) rewritten on skateboard (React + zero-crate Rust + SQLite) in `~/Projects/kiosk-skateboard`. Local repo only: branch `master`, no remote, nothing pushed, nothing deployed. Date: 2026-10-30.

## Result

- Parity: 47 of 47 portable items done, 12 framework-only items not ported on purpose (PARITY.md, `[-]` rows with reasons). 0 open
- HTTP parity vs Rails run from a copy: 49 of 50 checks match; the 1 difference is the new `/api/health`
- Scripted browser run on both apps: identical totals, visibility and saved cart at every step
- Tests: 337 Rust passed (279 template + 58 kiosk), 1 ignored (template live-Stripe test); 57 front-end passed (25 kiosk + 32 template). 0 failed
- `cargo build --release --locked`: ok (binary 1.0 MB). `[dependencies]` empty; `Cargo.lock` holds only `skateboard-backend`
- `docker build --pull`: ok from a clean `git archive` of `232ffb3`, image `kiosk-skateboard:232ffb3` (`sha256:1fc1d4b9161a...`), 141,648,913 bytes (142 MB, 36 MB gzipped) vs Rails image 817 MB. Smoke run: `/` 200, `/menu_items` 19 items. Idle memory 7 MB vs 135 MB (Rails), same machine
- No new crates or npm packages. One stop recorded in BLOCKERS.md (Vitest)

## Commits (author Steve Derico, GitHub noreply address)

| SHA | Change |
|---|---|
| `e50ff0f` | PARITY.md inventory |
| `7f36150` | Scaffold with create-skateboard-app 2.1.1 (skateboard 5.10.0) |
| `91e16d7` | Rust kiosk routes, schema, seeds, Rails encodings |
| `5f7405f` | React kiosk page on the skateboard shell |
| `232ffb3` | Dockerfile, parity scripts, docs, this report |
| next | Image ID recorded in this report |

## How it was verified

1. Rails copy: `~/Projects/kiosk` copied to `~/Projects/kiosk-rails-compare`, built with its own Dockerfile, run on 127.0.0.1:38080 (`SECRET_KEY_BASE` random). The original repo was not modified
2. Live: read-only GETs to kiosk.bixbyapps.com (`/`, `/menu_items`, static files, `/up`, 404s). Live matches the repo (only the CDN's injected script differs). No POSTs to production
3. This app: Docker image on 127.0.0.1:38200 with a named volume at `/app/backend/databases`
4. `scripts/parity/http-parity.sh` (50 requests, both apps), `scripts/parity/ui-scenario.sh` (Brave via agent-browser, both apps), screenshots of menu, edit dialog and checkout compared side by side
5. Rails behavior captured into tests: `allow_browser :modern` over 32 user agents, decimal casting of `total`, ActiveSupport float/string encoding, stored order rows, schema DDL from `sqlite_master`
6. Persistence: order created, container restarted (no re-seed, orders kept), stopped (WAL checkpointed), `PRAGMA integrity_check` = ok on the copied DB
7. Browser console: no errors on the page flows

## Decisions for Steve

1. **`POST /orders`.** Rails returns 422 to every request (CSRF on, and the page never gets a token), so orders are never saved in production; the kiosk page never calls it either. Built: same-origin browser requests create the order (Rails' exact create logic and response); curl, cross-site and no-Origin requests get the same 422 page as Rails. Alternative: always 422 (strict). Say which
2. **Front-end tests.** Brief said Vitest; house rules and skateboard use `node --test`. Used `node --test` (BLOCKERS.md). Keep, or approve Vitest + a DOM library
3. **Font.** Switched the kiosk page to Tailwind v3's system font stack so it looks like the Rails page, instead of skateboard's Geist. Keep parity or switch to Geist
4. **Analytics.** Kept the legacy analytics tag (same host, site id and domain lock as the Rails page). House default for new apps is apex-analytics; moving is a separate call
5. **Data.** Rails had no volume: every deploy reset the menu and orders. This app keeps them on a volume (as asked). The menu only seeds on an empty volume, so later seed changes need a manual update or an empty volume
6. **`JWT_SECRET`.** Skateboard will not start in production without one (32+ chars), even though the kiosk has no login. Set a random value
7. **Skateboard version.** The published CLI scaffolds 5.10.0 / ui 5.4.0; skateboard master is 5.13.0 / ui 5.11.0. Update later with `scripts/update-skateboard.js` if wanted
8. **Template API.** Signup, signin, `/api/me`, checkout, portal, payment webhook stay in the code (for template updates) but answer 404 through the kiosk router
9. **Headers.** Skateboard's set replaces Rails': `X-Frame-Options: DENY` (Rails `SAMEORIGIN`), a CSP that allows the analytics host and `data:` images, HSTS `max-age=31536000; includeSubDomains; preload` (Rails `max-age=63072000; includeSubDomains`). The `preload` token on a subdomain does nothing by itself
10. **Repo.** No remote yet. Name and visibility (private by default) are Steve's call; `stevederico/kiosk` holds the Rails history

## Proposed cutover (NOT executed)

The production cutover plan (host and service names, proxy settings, steps and rollback) is kept in internal notes outside this repo.

## Unknown (not checked)

- Production service names, the exact running image tag and its env var names: not read, production was out of scope
- Whether anything besides the page calls `POST /orders` (it always fails in production today, so likely nothing)
- Whether real kiosk hardware runs a browser older than Chrome 120 / Safari 17.2 (both apps answer 406)
- CDN cache or WAF rules specific to kiosk.bixbyapps.com
- Running in place on port 80 as uid 999 (not tested on the host)
- A bind mount owned 999:999 was not tested here (a named Docker volume was); the uid inside the image is 999

## Leftovers on this machine

- `~/Projects/kiosk-rails-compare` (copy of the Rails app used for comparison)
- Local images `kiosk-rails-compare:7c8be77`, `kiosk-skateboard:local`, `kiosk-skateboard:232ffb3`. Test containers, the test volume and the dev backend were removed
