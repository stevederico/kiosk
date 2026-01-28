# Kiosk

<img width="2555" height="1334" alt="Screenshot 2025-07-17 at 8 59 43 PM" src="https://github.com/user-attachments/assets/7241eeed-48fa-4400-ac20-b19fa8a02339" />

A modern Rails 8 self-service kiosk ordering application for food courts and quick-service restaurants. Built with Hotwire for real-time interactions without JavaScript complexity.

## Tech Stack

- **Rails 8.0.2** - Modern Ruby on Rails framework
- **Ruby 3.4.5** - Latest Ruby version
- **SQLite** - Lightweight embedded database
- **Hotwire** - Turbo and Stimulus for reactive UI
- **Solid Queue** - Database-backed Active Job adapter
- **Solid Cache** - Database-backed Rails cache
- **Kamal** - Zero-downtime Docker deployments
- **Thruster** - HTTP/2 asset caching and compression

## Prerequisites

- Ruby 3.4.5
- SQLite 3.8.0+
- Bundler
- Docker (for deployment)

## Setup

```bash
# Install dependencies
bundle install

# Setup database
bin/rails db:setup

# Start the server
bin/rails server
```

Visit `http://localhost:3000` to see the kiosk interface.

## Database

The app uses SQLite with separate databases for:
- Primary (menu items, orders)
- Cache (Solid Cache)
- Queue (Solid Queue)
- Cable (Action Cable)

### Seed Data

Default menu includes food court items (pizza, hotdogs, salads, etc.). Customize via `db/seeds.rb`.

## Deployment

### Docker

Build and run locally:

```bash
docker build -t kiosk .
docker run -d -p 80:80 -e RAILS_MASTER_KEY=<your-key> --name kiosk kiosk
```

### Kamal

Configure deployment settings in `config/deploy.yml`:

1. Update server IPs
2. Set domain and SSL settings
3. Configure registry credentials
4. Set secrets in `.kamal/secrets`

Deploy:

```bash
bin/kamal setup
bin/kamal deploy
```

**Important:** Never commit `config/master.key`. Store it securely and provide via environment or secrets manager.

## Configuration

- **Environment:** Configure via `config/environments/`
- **Credentials:** Use `rails credentials:edit` for encrypted secrets
- **Deploy:** Edit `config/deploy.yml` with your server details (placeholders provided)

## Development

```bash
# Console
bin/rails console

# Database console
bin/rails dbconsole

# Run migrations
bin/rails db:migrate
```

## License

MIT
