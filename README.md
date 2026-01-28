# Kiosk 🍕

<img width="2555" height="1334" alt="Screenshot 2025-07-17 at 8 59 43 PM" src="https://github.com/user-attachments/assets/7241eeed-48fa-4400-ac20-b19fa8a02339" />

A self-service food court kiosk built with Rails 8 and Hotwire. Touch-friendly UI, real-time updates, runs on SQLite.

## Quick Start 🚀

```bash
bundle install
bin/rails db:setup
bin/rails server
```

That's it. Visit `http://localhost:3000` and start ordering.

## What's Inside 📦

- **Rails 8.0.2** with Hotwire (Turbo + Stimulus)
- **SQLite** for everything (primary DB, cache, queue, cable)
- **Solid Queue** runs background jobs
- **Kamal** for zero-downtime deploys
- **Thruster** for HTTP/2 and asset compression

No JavaScript framework needed. No Redis required. Just Rails doing Rails things.

## Customize the Menu 🍔

Default seed data has Costco food court vibes (hotdog combos, pizza slices, churros). Edit `db/seeds.rb` to make it yours.

## Deploy 🐳

**Docker:**
```bash
docker build -t kiosk .
docker run -d -p 80:80 -e RAILS_MASTER_KEY=<key> kiosk
```

**Kamal:**
```bash
# Edit config/deploy.yml with your server details
bin/kamal setup
bin/kamal deploy
```

Don't commit `config/master.key` - ever.

## Development 🛠️

```bash
bin/rails console      # Rails console
bin/rails dbconsole    # SQLite console
bin/rails db:migrate   # Run migrations
```

## License

MIT
