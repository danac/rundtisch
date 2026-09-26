# Demo

## MySQL

The dev scripts expect a local MySQL container named `wasmer-mysql`, with database `rundtisch_dev` and user `demo` / `demo`. `docker start` and `docker stop` pause that container without deleting it. `docker ps` only lists running containers; `docker ps -a` shows it after a stop. `docker rm` deletes the container and the data inside it, because this `docker run` does not mount a volume. `MYSQL_DATABASE` creates `rundtisch_dev` only the first time the data directory is initialized. Add or drop a database later by running `mysql` as `root` in the running container, and `GRANT` `demo` access to any database you add.

```bash
sudo docker run --name wasmer-mysql \
  -e MYSQL_DATABASE=rundtisch_dev \
  -e MYSQL_USER=demo \
  -e MYSQL_PASSWORD=demo \
  -e MYSQL_ROOT_PASSWORD=root \
  -p 3306:3306 \
  mysql:8

sudo docker start wasmer-mysql
sudo docker stop wasmer-mysql
sudo docker ps -a --filter name=wasmer-mysql
sudo docker rm wasmer-mysql

sudo docker exec -it wasmer-mysql mysql -uroot -proot -e "SHOW DATABASES;"
sudo docker exec -it wasmer-mysql mysql -uroot -proot -e "
  CREATE DATABASE rundtisch_other;
  GRANT ALL PRIVILEGES ON rundtisch_other.* TO 'demo'@'%';
  FLUSH PRIVILEGES;
"
sudo docker exec -it wasmer-mysql mysql -uroot -proot -e "DROP DATABASE rundtisch_dev;"
```

## Wasmer

`wasmer.toml` and `app.yaml` in this folder are the Edge package and app drafts. The `[fs]` map mounts `web/dist` at `/app/web`. The native binary serves that directory as the SPA when the path exists (Edge, or `STATIC_DIR=...` locally). Split-dev keeps using Vite: `npm run dev` and `npm run dev:wasmer` do not set `STATIC_DIR`, and `/app/web` is not on the host, so Axum stays API-only.

Build the frontend and the WASIX modules before `wasmer deploy` (from the repository root):

```bash
npm run build --prefix demo/web
cargo wasix build --release
```

CI (`.github/workflows/ci.yml`) does the same on pushes to `main` and on `workflow_dispatch`: it installs wasix and wasmer-cli, replaces `Cargo.lock` with `Cargo.wasix.lock`, builds the frontend, builds the WASIX release, and deploys from this folder using the `Wasmer` GitHub environment (`WASMER_TOKEN`).

Then, from `demo/`:

```bash
wasmer deploy --owner YOUR_WASMER_USERNAME --no-persist-id
```

Leave `owner` commented in `app.yaml`. After the first deploy, set `AUTH_JWT_ACCESS_SECRET`, `AUTH_JWT_VERIFY_SECRET`, and `AUTH_HASH_PEPPER` as app secrets. Edge injects `DB_*` for the managed MySQL database; the server does not migrate on startup.

From the repository root, with MySQL running, apply migrations and start the API by running the release WASIX binaries. Run migrate first, then the server. Both need network access and the same database URL and `DB_SSL_MODE=required`. Raw `wasmer run` of the `.wasm` file does not apply `wasmer.toml` `[fs]` or `PORT=80`; Vite can still proxy `/api` to `localhost:8787`.

```bash
wasmer run target/wasm32-wasmer-wasi/release/migrate.wasi.wasm \
  --net \
  --env DATABASE_URL=mysql://demo:demo@127.0.0.1:3306/rundtisch_dev \
  --env DB_SSL_MODE=required

wasmer run target/wasm32-wasmer-wasi/release/native.wasi.wasm \
  --net \
  --env DATABASE_URL=mysql://demo:demo@127.0.0.1:3306/rundtisch_dev \
  --env DB_SSL_MODE=required
```
