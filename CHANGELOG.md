# Changelog

## v0.2.0 — 2026-09-16

### Added

- Shared address books with role-based access controls and administration in the web console.
- Device auto-registration endpoints and a Devices view for current RustDesk clients.
- Standalone HTTPS deployment with Caddy as an alternative to an existing Traefik proxy.

### Fixed

- Implemented the RustDesk 1.4.x `PUT /api/ab/peer/update/{guid}` contract: partial
  updates now persist peer metadata and replace tags only when tags are supplied.
- Accepted the stock client's bare JSON-array body for `DELETE /api/ab/peer/{guid}` while
  retaining the legacy object body.
- Returned the exact `SYSINFO_UPDATED` / `ID_NOT_FOUND` text responses expected by
  `hbbs_http::sync` for `/api/sysinfo`.
- Added `GET /api/login-options` returning an empty list, so current clients can probe for
  optional OIDC support without falling through to the SPA frontend.

### Security

- Pinned the example `hbbs`/`hbbr` images to `rustdesk-server:1.1.16` rather than an
  unreviewed moving `latest` tag.
- Made the example Compose configuration fail fast if `JWT_SECRET` or `ADMIN_PASSWORD` is
  absent; the address-book example binds locally by default for TLS reverse-proxy use.

### Documentation

- Added a production HTTPS deployment runbook for the existing a1 Traefik and Cloudflare
  DNS-01 setup, using `https://ab.rustdesk.rmg7.com` and no public port `21114`.
- Added a standalone Caddy HTTPS stack (`docker-compose.caddy.yml`, `Caddyfile`, and
  `.env.caddy.example`) with an operational deployment, validation, and rollback guide.
