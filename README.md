# Kiosk

Self-service food court kiosk ("Frankie Food Court Kiosk"): tap menu items, edit the receipt, check out on a mock PIN pad.

Rewrite of the Rails 8 kiosk (`~/Projects/kiosk`, commit `7c8be77`) on the skateboard stack: React + Vite front end, zero-crate Rust backend, SQLite. Scaffolded with `create-skateboard-app` 2.1.1 (skateboard 5.10.0), updated to skateboard 5.13.0 and skateboard-ui 5.11.0 with `scripts/update-skateboard.js`. Feature checklist: [PARITY.md](PARITY.md). Verification results: [REPORT.md](REPORT.md).

## Run

```bash
npm install
cd backend && cargo run     # Rust on :8000 (serves dist/ and the API)
npm start                   # Vite on :5173, proxies /menu_items and /orders to :8000
```

Production-like: `npm run build`, then `cd backend && cargo run` and open http://localhost:8000.

## Test

```bash
npm test                              # typecheck + node --test (src/lib/*.test.ts, template script tests)
cd backend && cargo test --locked     # Rust unit + route tests (src/kiosk/*)
cd backend && cargo build --release --locked
```

## Routes (same as the Rails app)

| Path | Result |
|---|---|
| `GET /` | Kiosk page (also `/kiosk.html`, `/kiosk`) |
| `GET /menu_items(.:format)` | `MenuItem.all` as Rails JSON (price as decimal string, ISO ms timestamps, `&` escaping) |
| `POST /orders(.:format)` | Creates an order: 201 `{"message":"Order created successfully","order_id":N}`. Same-origin browser requests only; others get Rails' 422 page. Body over 16 KiB: 413. More than 10 per minute from one IP: 429. A form key nested past 32 levels or over 4096 pairs: 400, like Rack (see REPORT.md) |
| `public/` files | `icon.png`, `icon.svg`, `images/frankie-logo.png`, `robots.txt`, `400/404/406/422/500.html` |
| anything else | 404 with Rails' `404.html` (including `/up`, which Rails never routed) |
| `GET /api/health` | Skateboard health check for the container (new; Rails had none) |

Old browsers get Rails' 406 page (`allow_browser versions: :modern`). The skateboard account and billing API (`/api/signup`, `/api/checkout`, ...) is closed: the kiosk has no login.

Code: `backend/src/kiosk/` (routes, schema/seeds, Rails encodings, browser check, params), `src/components/` and `src/lib/` (page).

## Data

- SQLite file `backend/databases/Kiosk.db` (`backend/config.json`). In the image: `/app/backend/databases/Kiosk.db`
- Mount a volume at `/app/backend/databases`. The image holds no data
- First boot on an empty volume creates the Rails schema (`menu_items`, `orders`, `order_items`, `schema_migrations`) and seeds the 19-item menu, like `db:prepare`. Later boots keep the data
- The skateboard template tables (`Users`, `Auths`, `WebhookEvents`) are also created; they stay empty
- The container runs as `skateboard` (uid/gid 999): a host bind mount must be owned 999:999

## Environment (names only)

| Name | Needed | Purpose |
|---|---|---|
| `NODE_ENV` | set to `production` by the image | Production mode (HSTS, compact logs, no `.env` loading) |
| `JWT_SECRET` | yes in production | Skateboard refuses to start without a 32+ char value. The kiosk issues no tokens; any random value works (`openssl rand -base64 48`) |
| `PORT` | no (default 8000) | Listen port |
| `STRIPE_KEY`, `STRIPE_ENDPOINT_SECRET` | no | Unused; the server logs a warning when unset |
| `TRUST_PROXY` | behind a proxy | Number of trusted reverse proxies in front of the app. The `POST /orders` limit (10 per minute) keys on the client IP taken that many hops from the right of `X-Forwarded-For` (every line of the header, joined in order); unset, every client behind the proxy shares the proxy's one bucket. Set it to the real hop count before deploying (REPORT.md, "Deployment") |
| `CORS_ORIGINS`, `FRONTEND_URL`, `LIBSQL_URL`, `FREE_USAGE_LIMIT`, `APP_NAME` | no | Template options, unused by the kiosk |
| `SKATEBOARD_BACKEND_DIR` | set by the image | Where `config.json` lives |

## Docker

```bash
D=$(mktemp -d); git archive HEAD | tar -x -C "$D"
docker build --pull --platform linux/amd64 -t kiosk:$(git rev-parse --short HEAD) "$D"
docker run -p 8000:8000 -v kiosk-data:/app/backend/databases -e JWT_SECRET="$(openssl rand -base64 48)" kiosk:<sha>
```

Base images are pinned by digest in the `Dockerfile`.

## Backups (proposal, not configured)

All data is one SQLite file, `Kiosk.db`, in the directory mounted at `/app/backend/databases`. To back it up:

- Copy it with `sqlite3 Kiosk.db ".backup /path/to/copy.db"` while the app runs, or copy the directory with the app stopped (the app uses WAL; a plain `cp` of a live file can miss recent writes)
- Restore test: start the image on a copy with a random `JWT_SECRET` and check `/api/health` answers 200 and `/menu_items` lists the menu

## License

MIT
