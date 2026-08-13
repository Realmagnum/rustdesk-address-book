# RustDesk Address Book — Maintained Fork

[![License: AGPL-3.0](https://img.shields.io/badge/License-AGPL--3.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.85%2B-orange.svg)](https://www.rust-lang.org)
[![Docker](https://img.shields.io/badge/Docker-ready-2496ED.svg)](Dockerfile)

A self-hosted **address book server for RustDesk** — an open-source replacement for the
paid `rustdesk-server-pro` address book / account feature. It implements the RustDesk Pro
API on port **21114**, which official clients use for address book sync, and works with
**stock RustDesk clients** — desktop (Sciter) and Android/iOS (Flutter) — no custom client
builds required.

> **Why this fork?** The upstream project [ds4a/rustdesk-address-book](https://github.com/ds4a/rustdesk-address-book)
> was abandoned (last commit February 2026) with its API emulation out of sync with current
> RustDesk clients: the address book tab failed to open on desktop and mobile. This fork
> restores client compatibility, unblocks building with modern toolchains, and hardens the
> server for production use.

---

## What's fixed and improved

### Client compatibility — verified against the client source code

| Commit | Fix |
|--------|-----|
| `e8eec86` | **Desktop client (Sciter, `ui/ab.tis`):** added `POST /api/ab/get` — the endpoint the client's `getAb()` actually calls, returning `{ updated_at, data }`; made `tag_colors` optional on update (clients never send it); `POST /api/currentUser` is now accepted (with the `verifier` field the client expects). |
| `c2543af` | **Android/iOS client (Flutter, `ab_model.dart`):** the mobile UI sends `POST` on *every* read endpoint (`/api/ab/settings`, `/api/ab/personal`, `/api/ab/shared/profiles`, `/api/ab/peers`, `/api/ab/tags/{guid}`), while the server only accepted `GET`. All read routes now accept both methods. |

### Build

- `d3dac06` — bumped the Docker build stage from `rust:1.83` (whose Cargo cannot parse
  crates requiring edition2024, e.g. `base64ct 1.8.3`) to `rust:1-bookworm`.
- `2bbb724` — fixed the BuildKit cache-mount pitfall (the release binary is copied out of
  the cache mount so it lands in the image layer).
- Added BuildKit cache mounts for cargo/npm — rebuilds after small changes take minutes
  instead of tens of minutes.

### Security hardening

- **Login rate limiting** — in-memory sliding window, 5 attempts/minute per username,
  `429 Too Many Requests` beyond that (reset on successful login).
- **Non-root container** — dedicated `rustdesk` system user (uid 10001) in the image;
  the compose file runs the service as `1000:1000` to match the host data directory owner.
- **CORS restricted to same-origin** — native RustDesk clients are not subject to CORS,
  so this does not affect them; it stops arbitrary websites from calling the API.
- **Honest configuration warnings** — the "no JWT secret" warning now fires only when the
  secret is genuinely missing (env/config resolved); a loud warning appears if the admin
  password falls back to the default `admin`.
- **LICENSE file** — full AGPL-3.0 text added (GitHub license detection now works).

### Tests

- `rate_limit_blocks_after_max_attempts`
- `rate_limit_resets_after_successful_login`
- `legacy_ab_accepts_client_payload_without_tag_colors` (client payload compatibility)

---

## Quick start

```bash
git clone https://github.com/Realmagnum/rustdesk-address-book.git
cd rustdesk-address-book
```

### Option A — full stack (new deployment)

The included [docker-compose.yml](docker-compose.yml) runs hbbs (rendezvous),
hbbr (relay) and the address book in one command:

```bash
cat > .env <<'EOF'
JWT_SECRET=$(openssl rand -hex 32)
ADMIN_PASSWORD=$(openssl rand -base64 18 | tr -d '=+/')
EOF
docker compose up -d
```

### Option B — address book only (existing RustDesk server)

The address book server is completely independent of hbbs/hbbr — run just the
`address-book` service from [docker-compose.yml](docker-compose.yml) on the same host:

```yaml
services:
  address-book:
    build: .
    container_name: rustdesk-address-book
    user: "1000:1000"            # non-root; match the owner of ./data
    ports:
      - "21114:21114"
    volumes:
      - ./data:/data
    environment:
      RUSTDESK_AB_DB_PATH: /data/db.sqlite3
      RUSTDESK_AB_JWT_SECRET: ${JWT_SECRET}
      RUSTDESK_AB_ADMIN_PASSWORD: ${AB_ADMIN_PASSWORD}
    restart: unless-stopped
```

```bash
docker compose up -d address-book
```

The web admin console and the API are served on `http://<host>:21114`; the first-run
admin account is created from `ADMIN_PASSWORD` / `AB_ADMIN_PASSWORD`.

## Client setup

Set the ID/Relay server in each RustDesk client (e.g. `rustdesk.example.com`). The client
derives the API server as `<server>:21114` automatically (`get_api_server` in the client
source) — sign in with an account created in the web console and the address book syncs.

| Client | Endpoints used (verified in client source) |
|--------|---------------------------------------------|
| Desktop (Sciter, `ui/ab.tis`) | `POST /api/ab/get`, `POST /api/ab`, `POST /api/currentUser` |
| Android/iOS (Flutter, `ab_model.dart`) | `POST` `/api/ab/settings`, `/api/ab/personal`, `/api/ab/shared/profiles`, `/api/ab/peers`, `/api/ab/tags/{guid}`, `/api/ab/peer/add/{guid}`, `/api/ab/peer/update/{guid}`, `/api/ab/peer/{guid}` |

## Running tests

```bash
docker build --target backend -t ab-backend-test .
docker run --rm --entrypoint cargo ab-backend-test test --release --manifest-path /app/Cargo.toml
```

## Security notes

- Port `21114` must be reachable by clients; put the server behind a firewall / reverse
  proxy if you need to restrict access.
- Back up `./data/db.sqlite3` — it holds the entire address book (peers, tags, users).
- The admin password and JWT secret are read from the environment; never commit them.

## License

[AGPL-3.0](LICENSE) — derivative work of [ds4a/rustdesk-address-book](https://github.com/ds4a/rustdesk-address-book),
licensed under the same terms.
