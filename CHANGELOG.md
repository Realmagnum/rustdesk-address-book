# Changelog

## Unreleased

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
