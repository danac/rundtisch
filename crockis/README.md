# Crockis

Private photo library. Auth is a rundtisch Axum API, modelled on the demo app. Collections and pictures are stored in MySQL; image files live on the `data` volume mounted at `/data`.

```
crockis/
├── api/              # Axum app: auth routes, native + migrate + seed + auth-link
├── web/              # Vite + React SPA
├── wasmer.toml       # Wasmer package (WASIX binaries + web/dist)
├── app.yaml          # Wasmer Edge app (name: crockis-photos)
└── package.json      # concurrently; `npm run dev`
```

## Commands

```bash
npm install --prefix crockis
npm install --prefix crockis/web
npm run dev --prefix crockis
```

| Process | URL | Role |
|---------|-----|------|
| Vite | http://localhost:5174 | SPA — open this in the browser |
| API | http://localhost:8788 | `crockis-native` (proxied at `/api`) |

Create the MySQL database once (`demo` / `demo`, same server as the demo):

```bash
mysql -h 127.0.0.1 -P 3306 -udemo -pdemo -e 'CREATE DATABASE IF NOT EXISTS crockis_dev'
DATA_DIR=crockis/data \
DATABASE_URL=mysql://demo:demo@127.0.0.1:3306/crockis_dev \
  AUTH_HASH_PEPPER=cccccccccccccccccccccccccccccccc \
  cargo run -p crockis --bin crockis-migrate
DATA_DIR=crockis/data \
DATABASE_URL=mysql://demo:demo@127.0.0.1:3306/crockis_dev \
  cargo run -p crockis --bin crockis-seed --features seed
```

`crockis-migrate` applies auth, object-storage, and library migrations and, when both bootstrap admin variables are set, upserts that admin. `crockis-seed` downloads the catalog into the local object store (`STORAGE_LOCAL_ROOT`, then `DATA_DIR`, then `/data`). Re-running it leaves pictures whose files already match their BLAKE3 digest and restores a missing file only when the new bytes match. The server does not seed.

`npm run dev` sets `DATA_DIR` to `crockis/data` (gitignored). It also expects `AUTH_HASH_PEPPER`, WebAuthn RP id `localhost`, origin `http://localhost:5174`, name `crockis-photos`, and placeholder SMTP variables so the process can start. Real recovery mail needs a reachable SMTP server. Mint links with:

```bash
cargo run -p crockis --bin crockis-auth-link -- invite --email user@example.com
cargo run -p crockis --bin crockis-auth-link -- recover --email user@example.com
```

## Deploy

Wasmer Edge app `crockis-photos` (`https://crockis-photos.wasmer.app`): WASIX binaries, managed MySQL in `fr-roub1`, a `data` volume mounted at `/data`, and a post-deploy `migrate` job. That job does not download pictures. After deploy, load the catalog over SSH (the app's database env and `/data` are already set):

```bash
wasmer ssh "$WASMER_OWNER/crockis-photos" -- seed
```

The volume is created by the next `wasmer deploy` (there is no separate volume command, and the live app has none until then). Keep the volume name `data`; renaming it deletes the files. Workflow **Deploy Crockis on Wasmer Edge** (`.github/workflows/deploy-crockis.yml`) matches the demo deploy: every push and `workflow_dispatch`, GitHub environment **Wasmer** (`WASMER_TOKEN`, `WASMER_OWNER`). It builds `crockis-seed` as well as the server and migrate binaries.

See [api/README.md](api/README.md) and [web/README.md](web/README.md).
