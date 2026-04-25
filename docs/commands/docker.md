# metaphor-dev docker

Local `docker compose` lifecycle. Operates on the compose file and env file declared for a *local* environment in [`metaphor.deploy.yaml`](../reference/configuration.md#metaphordeployyaml). Local environments are those without a `host:` field — for remote targets use [`deploy`](deploy.md) instead.

> **Invocation:** examples below use the standalone plugin form `metaphor-dev docker …`. When invoked via the core CLI, drop the `-dev` suffix: `metaphor docker …`. Both routes are equivalent.

---

## Subcommands

| Subcommand | Description |
|------------|-------------|
| [`docker up`](#docker-up) | Start the stack (`docker compose up`) |
| [`docker down`](#docker-down) | Stop and remove the stack (`docker compose down`) |
| [`docker logs`](#docker-logs) | Tail compose logs |
| [`docker ps`](#docker-ps) | Show running containers |
| [`docker restart`](#docker-restart) | Restart a single service |
| [`docker pull`](#docker-pull) | Pull images defined in compose |
| [`docker build`](#docker-build) | Build images defined in compose |

All subcommands accept `--env <name>` (default: `dev`).

---

## Resolution

Every `docker` subcommand:

1. Loads `metaphor.deploy.yaml` by walking up from the current directory.
2. Looks up `environments.<env>` (defaulting to `dev`).
3. Refuses to run if that environment has a `host:` field — those are remote targets and belong to [`metaphor-dev deploy`](deploy.md).
4. Resolves the compose file from `environments.<env>.compose_file` (falls back to `defaults.compose_file`, then `deployment/compose.yaml`).
5. Resolves the env file from `environments.<env>.env_file` (falls back to `.env.<env>`).
6. Shells out to `docker compose -f <compose> --env-file <env> …` from the workspace root.

A missing env file logs a warning but is not fatal; a missing compose file errors out.

---

## docker up

Bring the local stack online.

### Synopsis

```
metaphor-dev docker up [OPTIONS]
```

### Options

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--env` | string | `dev` | Environment name from `metaphor.deploy.yaml` |
| `--attach` | bool | `false` | Run in the foreground (default is detached, i.e. `-d`) |
| `--build` | bool | `false` | Build images before starting (`docker compose up --build`) |
| `--service` | string (repeatable) | none | Limit to specific services |

### Examples

Start the full dev stack in the background:

```sh
metaphor-dev docker up
```

Rebuild images first, then start only the API service:

```sh
metaphor-dev docker up --build --service bersihir-service
```

Run in the foreground (Ctrl-C to stop):

```sh
metaphor-dev docker up --attach
```

---

## docker down

Stop and remove the stack.

### Synopsis

```
metaphor-dev docker down [OPTIONS]
```

### Options

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--env` | string | `dev` | Environment name |
| `--volumes` | bool | `false` | Also remove named volumes (**destructive** — wipes Postgres data, MinIO buckets, etc.) |

### Examples

Stop and remove containers, keeping volumes:

```sh
metaphor-dev docker down
```

Tear everything down including data volumes:

```sh
metaphor-dev docker down --volumes
```

---

## docker logs

Tail compose logs.

### Synopsis

```
metaphor-dev docker logs [OPTIONS]
```

### Options

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--env` | string | `dev` | Environment name |
| `-f`, `--follow` | bool | `false` | Follow output |
| `--tail` | string | `200` | Number of lines from the end (passed straight to `docker compose logs --tail`) |
| `--service` | string | none | Limit to a single service |

### Examples

Show last 200 lines from every service:

```sh
metaphor-dev docker logs
```

Follow logs from one service:

```sh
metaphor-dev docker logs --follow --service bersihir-service
```

---

## docker ps

List running containers in the stack (`docker compose ps`).

### Synopsis

```
metaphor-dev docker ps [--env <name>]
```

---

## docker restart

Restart a single service.

### Synopsis

```
metaphor-dev docker restart --service <name> [--env <name>]
```

### Options

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--env` | string | `dev` | Environment name |
| `--service` | string | required | Service name to restart |

### Examples

```sh
metaphor-dev docker restart --service bersihir-service
```

---

## docker pull

Pull images declared in the compose file.

### Synopsis

```
metaphor-dev docker pull [--service <name>]... [--env <name>]
```

### Examples

Pull every image referenced by compose:

```sh
metaphor-dev docker pull
```

Pull only specific services:

```sh
metaphor-dev docker pull --service postgres --service redis
```

---

## docker build

Build images declared in the compose file.

### Synopsis

```
metaphor-dev docker build [--push] [--service <name>]... [--env <name>]
```

### Options

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--env` | string | `dev` | Environment name |
| `--push` | bool | `false` | Run `docker compose push` after a successful build |
| `--service` | string (repeatable) | none | Limit to specific services |

### Examples

Build all images:

```sh
metaphor-dev docker build
```

Build the API image and push to the registry:

```sh
metaphor-dev docker build --service bersihir-service --push
```

> Note: `docker build` here is bound by what `compose.yaml` declares. To build with the same image-tagging rules used by `deploy push` (per-image registry, build args, multi-arch), use [`metaphor-dev deploy push <env>`](deploy.md#deploy-push) — even with `--skip-migrate` if you only want the build/push half.

---

## Configuration

`docker` commands read [`metaphor.deploy.yaml`](../reference/configuration.md#metaphordeployyaml) at the workspace root. Only the fields below are consulted (the rest are used by `deploy`):

| Field | Source | Purpose |
|-------|--------|---------|
| `environments.<env>.compose_file` | per-env, falls back to `defaults.compose_file` | Path to the compose file |
| `environments.<env>.env_file` | per-env, falls back to `.env.<env>` | Path to the `--env-file` |
| `environments.<env>.host` | per-env | If present, this env is **remote** and `docker` refuses to operate on it |

A minimal `metaphor.deploy.yaml` for local-only development:

```yaml
version: 1
defaults:
  compose_file: deployment/compose.yaml
environments:
  dev:
    env_file: deployment/.env.dev
    images: {}        # required key, can be empty for docker-only use
```

---

## Troubleshooting

### `compose file not found at …`

The path resolved from `compose_file` doesn't exist. Either fix the path in `metaphor.deploy.yaml` or create the file. Paths are resolved relative to the workspace root (the directory containing `metaphor.deploy.yaml`).

### `environment 'X' is remote (host: Y) — use \`metaphor deploy\` instead`

The named environment has a `host:` field. Either:

- pass a different `--env` that has no host, or
- use `metaphor-dev deploy` for remote operations.

### Warning: `env file not found at …`

`docker compose` will run without `--env-file`. Defaults baked into the compose file still apply, but per-env values won't. Create the file or point `env_file` at one that exists.

### `failed to spawn \`docker\``

The `docker` binary isn't on `PATH`. Install Docker Desktop (macOS/Windows) or `docker-ce` (Linux) and re-run.

---

## See Also

- [deploy](deploy.md) — Remote deployment commands that share `metaphor.deploy.yaml`
- [Configuration Reference](../reference/configuration.md#metaphordeployyaml) — Full schema for `metaphor.deploy.yaml`
- [dev serve](dev.md#dev-serve) — Run the app via `cargo run` instead of compose
