# HTTPS deployment on a1

This guide deploys `rustdesk-address-book` behind the existing Traefik instance on `a1`.
It protects the account, address-book and device-inventory API on HTTPS without changing
RustDesk rendezvous (`hbbs`) or relay (`hbbr`) traffic.

## Why a separate hostname

Use `ab.rustdesk.rmg7.com` for the address-book API.

Do **not** multiplex the HTTP API with `rustdesk.rmg7.com:21114`. Current RustDesk clients
already support an explicitly configured API URL, while a dedicated hostname lets Traefik
terminate TLS on `443` and removes public port `21114`.

Client settings:

```text
ID Server:    rustdesk.rmg7.com
Relay Server: rustdesk.rmg7.com
API Server:   https://ab.rustdesk.rmg7.com
Key:          <contents of /opt/docker/rustdesk/data/id_ed25519.pub>
```

For the RustDesk client this is independent of the TCP/UDP ports used by `hbbs`/`hbbr`.

## Prerequisites

- `ab.rustdesk.rmg7.com` resolves to `a1`.
- Traefik is serving the `websecure` entry point and has the Cloudflare DNS-01 resolver
  `letsencrypt`.
- `/opt/docker/rustdesk-address-book/.env` has persistent `JWT_SECRET` and
  `AB_ADMIN_PASSWORD` values and mode `0600`.

## Compose change

Use a shared Docker network instead of publishing the API to the Internet:

```yaml
services:
  address-book:
    image: rustdesk-address-book:dev
    container_name: rustdesk-address-book
    user: "1000:1000"
    expose:
      - "21114"
    volumes:
      - ./data:/data
    environment:
      - RUSTDESK_AB_DB_PATH=/data/db.sqlite3
      - RUSTDESK_AB_JWT_SECRET=${JWT_SECRET:?set a persistent random JWT_SECRET in .env}
      - RUSTDESK_AB_ADMIN_PASSWORD=${AB_ADMIN_PASSWORD:?set a strong ADMIN_PASSWORD in .env}
      - RUSTDESK_AB_AUTO_ADD_DEVICES=${AUTO_ADD_DEVICES:-1}
    networks:
      - default
      - traefik-public
    restart: unless-stopped

networks:
  traefik-public:
    external: true
```

`expose` is documentation-only; the service becomes reachable to Traefik over
`traefik-public` as `http://rustdesk-address-book:21114`.

## Traefik router

Create `/opt/docker/traefik/rules/rustdesk-address-book.yml`:

```yaml
http:
  routers:
    rustdesk-address-book:
      rule: "Host(`ab.rustdesk.rmg7.com`)"
      entryPoints:
        - websecure
      tls:
        certResolver: letsencrypt
      service: rustdesk-address-book

  services:
    rustdesk-address-book:
      loadBalancer:
        servers:
          - url: "http://rustdesk-address-book:21114"
```

The existing Traefik configuration uses global HTTP-to-HTTPS redirect and therefore the
router must be `websecure`-only.

## Deployment

1. Create the DNS record (`ab.rustdesk.rmg7.com` -> `a1.rmg7.com`) in Cloudflare.
2. Validate the compose file with non-secret placeholder values:

   ```bash
   cd /opt/docker/rustdesk-address-book
   docker compose config --quiet
   ```

3. Replace the compose file and add the Traefik rule.
4. Recreate only the address-book service:

   ```bash
   cd /opt/docker/rustdesk-address-book
   docker compose up -d --force-recreate address-book
   ```

   Traefik watches its rules directory and loads the new router automatically. If the first
   ACME DNS-01 attempt reports TXT `NXDOMAIN`, wait for propagation and restart Traefik once
   to retry certificate issuance.

5. In every RustDesk client set the API Server to:

   ```text
   https://ab.rustdesk.rmg7.com
   ```

   RustDesk only accepts `http://` or `https://` in the API Server field.

## Verification

```bash
# Address-book must no longer listen on every public interface.
ss -ltnp | grep ':21114' || true

# Traefik reaches the internal service.
docker exec traefik wget -qO- http://rustdesk-address-book:21114/api/login-options

# HTTPS returns an actual API response, not the SPA fallback.
curl -fsS https://ab.rustdesk.rmg7.com/api/login-options

# Confirm the certificate is issued by Let's Encrypt.
echo | openssl s_client -connect ab.rustdesk.rmg7.com:443 \
  -servername ab.rustdesk.rmg7.com 2>/dev/null \
  | openssl x509 -noout -issuer -subject -dates

# Verify a client can log in and load its address book after changing API Server.
```

Expected response from `/api/login-options` is `[]`.

## Rollback

1. Restore `ports: ["21114:21114"]` and remove the `traefik-public` network from the
   address-book compose file.
2. Remove `/opt/docker/traefik/rules/rustdesk-address-book.yml`.
3. Recreate `address-book`:

   ```bash
   cd /opt/docker/rustdesk-address-book
   docker compose up -d --force-recreate address-book
   ```

No SQLite data is changed by this procedure; `/opt/docker/rustdesk-address-book/data` is
preserved throughout.
