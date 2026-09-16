# HTTPS deployment with Caddy

This is a self-contained alternative to Traefik for a new RustDesk Address Book
installation. Caddy obtains and renews a public Let's Encrypt certificate automatically,
then proxies the RustDesk API to the private `address-book` container.

Use this stack only on a host where TCP `80` and `443` are free. Do not run it alongside
Traefik, Nginx, Apache, or another reverse proxy that already owns those ports.

## Prerequisites

- A public DNS `A` or `AAAA` record, for example `ab.example.com`, pointing directly to
  this host. Do not use a Cloudflare orange-cloud proxy unless its SSL mode is **Full
  (strict)**.
- Inbound TCP `80` and `443` allowed by the host firewall and cloud security group.
- Docker Engine with Docker Compose plugin.

## Install

```bash
git clone https://github.com/Realmagnum/rustdesk-address-book.git
cd rustdesk-address-book
cp .env.caddy.example .env
chmod 600 .env
```

Edit `.env` and set these values:

```dotenv
ADDRESS_BOOK_DOMAIN=ab.example.com
ACME_EMAIL=admin@example.com
JWT_SECRET=<output of openssl rand -hex 32>
AB_ADMIN_PASSWORD=<strong unique password>
AUTO_ADD_DEVICES=0
```

Start the stack:

```bash
docker compose -f docker-compose.caddy.yml up -d --build
```

Caddy performs HTTP-01 ACME validation on port `80`, then serves HTTPS on `443`.
Its certificates are stored in the named `caddy_data` volume and survive container
recreation.

## RustDesk client configuration

Configure each client explicitly:

```text
ID Server:    rustdesk.example.com
Relay Server: rustdesk.example.com
API Server:   https://ab.example.com
Key:          <RustDesk Server public key>
```

The address book does not replace `hbbs` or `hbbr`. Their RustDesk TCP/UDP ports remain
configured independently.

## Verification

```bash
# Compose interpolation and schema check.
docker compose -f docker-compose.caddy.yml config --quiet

# Both containers must be running.
docker compose -f docker-compose.caddy.yml ps

# API must be served via HTTPS. An empty JSON array is expected.
curl --fail --silent https://ab.example.com/api/login-options

# Verify a public CA certificate and its name.
echo | openssl s_client -connect ab.example.com:443 -servername ab.example.com 2>/dev/null \
  | openssl x509 -noout -issuer -subject -dates

# Confirm address-book is not published directly on port 21114.
docker compose -f docker-compose.caddy.yml port address-book 21114
# Expected: no published port
```

## Operations

```bash
# Live Caddy logs, including certificate issuance/renewal.
docker compose -f docker-compose.caddy.yml logs -f caddy

# Rebuild after source changes.
docker compose -f docker-compose.caddy.yml up -d --build address-book

# Back up the address-book database before upgrades.
sqlite3 data/db.sqlite3 '.backup "data/db.sqlite3.backup"'
```

## Troubleshooting

- **Certificate issuance fails**: first check public DNS and that port `80` reaches Caddy.
  `docker compose -f docker-compose.caddy.yml logs caddy` reports ACME failures.
- **Cloudflare returns redirect loops or TLS errors**: use **Full (strict)** mode, or set
  the DNS record to DNS-only. Never use Flexible mode because it terminates TLS at
  Cloudflare and talks plain HTTP to the origin.
- **Port bind error**: another service owns `80` or `443`. Either stop/reconfigure that
  service or use the existing proxy integration documented in
  [`https-deployment.md`](https-deployment.md).
- **RustDesk shows the public cloud UI**: ensure API Server is explicitly set to
  `https://ab.example.com`; ID/Relay fields alone derive an API URL on port `21114`.

## Rollback

```bash
docker compose -f docker-compose.caddy.yml down
```

This stops containers only. It preserves `data/`, `caddy_data`, and `caddy_config`.
To remove certificate state as well, explicitly remove the named Caddy volumes after
confirming that no other Caddy deployment uses them.
