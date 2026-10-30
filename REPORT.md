# Kiosk rewrite report

Rails 8 kiosk (`~/Projects/kiosk` @ `7c8be77`, live at kiosk.bixbyapps.com) rewritten on skateboard 5.13.0 / skateboard-ui 5.11.0 (React + zero-crate Rust + SQLite) in `~/Projects/kiosk-skateboard`. Local repo only: branch `master`, no remote, nothing pushed, nothing deployed. Date: 2026-10-30.

## Result

- Parity: 47 of 47 portable items done, 12 framework-only items not ported on purpose (PARITY.md, `[-]` rows with reasons). 0 open
- HTTP parity vs Rails run from a copy: 49 of 50 checks match; the 1 difference is the new `/api/health`
- Scripted browser run on both apps: identical totals, visibility and saved cart at every step
- Tests: 337 Rust passed (279 template + 58 kiosk), 1 ignored (template live-Stripe test); 57 front-end passed (25 kiosk + 32 template). 0 failed. After the 5.13.0 upgrade: 342 Rust (284 + 58), 57 front end (see the last section)
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
| `0d9316c` | Image ID recorded in this report |
| `cfec801` | 0.6.0: update to skateboard 5.13.0 / skateboard-ui 5.11.0 |
| next | PARITY.md and REPORT.md results for 5.13.0 |

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
7. **Skateboard version.** Done: updated from 5.10.0 / ui 5.4.0 to 5.13.0 / ui 5.11.0 (see "Upgrade to 5.13.0")
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
- Local images `kiosk-rails-compare:7c8be77`, `kiosk-skateboard:local`, `kiosk-skateboard:232ffb3`, `kiosk-skateboard:cfec801`. Test containers, the test volume and the dev backend were removed
- Not mine, left running: container `qa-kiosk-gate3` (image `qa-kiosk:0d9316c`)

## Upgrade to 5.13.0

Done 2026-10-30, approved by Steve at 8:56 AM PT. Local only: no remote, no push, no tag, no deploy.

**Versions now in use**

- skateboard 5.13.0 (`skateboardVersion`), source `~/Projects/skateboard` at `f112fb7` (clean, already up to date with `origin/master`; `git pull --ff-only` was a no-op)
- `@stevederico/skateboard-ui` 5.11.0 (exact pin, `npm run verify:ui` ok)
- App version 0.5.0 → 0.6.0 (`package.json`, `src/constants.json`, CHANGELOG.md)

**How it was applied**

- `SKATEBOARD_REPO=~/Projects/skateboard node scripts/update-skateboard.js --yes`: 3-way merge, baseline tag 5.10.0, 0 conflicts
- The updater re-added `public/theme-init.js`, which the kiosk removed on purpose (it has no theme switch). Deleted again
- `docs/GUIDE.md`, `docs/UPGRADE.md`, `skills/skateboard/SKILL.md` are not on the updater's list. Their 5.10.0 → 5.13.0 template diff applied cleanly with `git apply --3way`
- `Dockerfile`: the 5.11.0 template change (copy `src/constants.json` into the image) was already in the kiosk Dockerfile. Base images stay pinned by digest

**Notable changes pulled in**

- 5.11.0: the image ships `src/constants.json` so checkout and the webhook can read the plan list (already present here)
- 5.12.0: Stripe `customer.subscription.*` events re-fetch the subscription from Stripe instead of trusting the event
- 5.12.0: an event for an old subscription no longer re-locks a user who subscribed again
- 5.12.0: a webhook DB error answers 500 without recording the event, so Stripe retries
- 5.12.0: `Users.subscription_id` column, added on startup to existing databases
- ui 5.5.0: 44px tap targets on tab bar items, theme toggle and Sign Out
- ui 5.6.0 / 5.8.0: Sheet `onUserClose` and touch scrolling
- ui 5.7.0 / 5.9.0 / 5.10.0 / 5.11.0: Sign Up shows server errors, opens first unless the device signed in before, hints at mistyped mail domains, asks `GET /signup/options` for an optional Turnstile key
- Effect on the kiosk: none visible. The kiosk router closes the template account and payment API (404), and the page uses no tab bar, Sheet, theme toggle or sign-up. The only kiosk-side effect is the empty `Users` table gaining the new column

**Files changed** (`cfec801`)

- Template, by the updater: `backend/src/routes.rs`, `backend/src/db.rs` (both now byte-identical to 5.13.0), `AGENTS.md` (version block), `package.json`, `package-lock.json` (only the skateboard-ui entry)
- Template docs, by `git apply`: `docs/GUIDE.md`, `docs/UPGRADE.md`, `skills/skateboard/SKILL.md`
- App: `CHANGELOG.md`, `README.md`, `AGENTS.md` (kiosk section), `src/constants.json`
- Untouched: everything in `backend/src/kiosk/`, `backend/src/main.rs`, `src/components/`, `src/lib/`, `index.html`, `vite.config.ts`, `Dockerfile`, `Cargo.toml`, `Cargo.lock`

**Dependencies**

- Crates: none added. `[dependencies]` still empty; `Cargo.lock` unchanged; nothing for BLOCKERS.md
- npm: no new packages. Only change: `@stevederico/skateboard-ui` 5.4.0 → 5.11.0 (from the template's documented pin); no new transitive packages in the lockfile
- `npm audit` shows 2 high findings (vite 8.2.0-8.2.2, source-map-js). They are the same before and after the upgrade and the template has the same pins. Not changed here

**Results**

- `npm test`: typecheck clean, 57 passed, 0 failed, `test:docs` ok
- `cargo test --locked`: 342 passed (5 new template webhook/subscription tests), 0 failed, 1 ignored (live Stripe)
- `cargo clippy --all-targets`: 14 warnings, all in vendored template files, same count as upstream 5.13.0; none in `src/kiosk/`
- `cargo build --release --locked`: ok, 14.6 s, binary 1,031,904 bytes
- `docker build --pull --platform linux/amd64` from a clean `git archive` of `cfec801`: ok, 22 s. Image `kiosk-skateboard:cfec801`, `sha256:fe8f468c4daf...`, 141,666,977 bytes (142 MB, 35.7 MB gzipped), 18 KB more than before
- Container on 127.0.0.1:38200 with a named volume: health 200, 19 menu items, same-origin order created (201, `order_id` 1), restart kept the order and did not re-seed, WAL checkpointed on stop, `PRAGMA integrity_check` ok. `/signup/options` (new ui probe) answers 404, as the template expects
- `scripts/parity/http-parity.sh` against the Rails copy (`kiosk-rails-compare:7c8be77` restarted on 127.0.0.1:38080): 49 of 50 SAME. The only DIFF is `GET /api/health` (added on purpose), same as before the upgrade
- `scripts/parity/ui-scenario.sh` on both apps: identical at every step except the known one (the grid unmounts at checkout; looks the same on screen)
- Menu screenshot, Rails vs this app: same layout, colors and totals. The upgrade did not change the page's look (the only change under `src/` is the version string)
- Test containers and the volume were removed afterwards

**Broke or needs Steve**

- Nothing broke
- Pre-existing, not from the upgrade: the heading "Welcome to the Frankie Food Court Kiosk!" wraps at different words than Rails, because it uses `text-balance` (house rule). Same text and size. Keep it or match Rails' line breaks
- Pre-existing, not from the upgrade: two menu clicks fired by a script in the same JS task add only the second item (the cart update reads state from before the first click). Real taps are separate events and work (the scripted scenario clicks one at a time and matches Rails). A functional state update would close it
- On deploy, an existing volume's `Users` table gets the `subscription_id` column on first boot. Automatic and harmless (the table is empty)
