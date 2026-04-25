# metaphor-dev deploy

Remote deployment lifecycle for environments declared in [`metaphor.deploy.yaml`](../reference/configuration.md#metaphordeployyaml). Operates on environments that have a `host:` field — for purely local stacks use [`docker`](docker.md) instead.

The model is intentionally thin: each command is a deterministic combination of `docker buildx`, `scp`, `ssh`, and `docker compose`. There is no bespoke orchestration layer or state store.

> **Invocation:** examples below use the standalone plugin form `metaphor-dev deploy …`. When invoked via the core CLI, drop the `-dev` suffix: `metaphor deploy …`. Both routes are equivalent.

---

## Subcommands

| Subcommand | Description |
|------------|-------------|
| [`deploy push`](#deploy-push) | Build, push to registry, and roll out a release |
| [`deploy rollback`](#deploy-rollback) | Roll back to a registry tag already pushed |
| [`deploy status`](#deploy-status) | `docker compose ps` over SSH |
| [`deploy logs`](#deploy-logs) | `docker compose logs` over SSH |
| [`deploy migrate`](#deploy-migrate) | Run database migrations against the remote env |
| [`deploy exec`](#deploy-exec) | Delegate to the workspace's infra project (legacy) |

---

## Resolution

Every `deploy` subcommand:

1. Loads `metaphor.deploy.yaml` by walking up from the current directory.
2. Looks up `environments.<env>` (where `<env>` is a positional argument, no default).
3. Refuses to run if that environment has no `host:` — those are local and belong to [`docker`](docker.md).
4. Computes the SSH target as `<ssh_user>@<host>` (falling back to `defaults.ssh_user`, then no user prefix).
5. Resolves the remote `deploy_dir` from `environments.<env>.deploy_dir` (falling back to `defaults.deploy_dir`).
6. Resolves the remote compose file and env file relative to that `deploy_dir`.

---

## deploy push

Build images, push them to the registry, transport the env file, and roll out the new tag on the remote host.

### Synopsis

```
metaphor-dev deploy push <ENV> [OPTIONS]
```

### Arguments

| Argument | Description |
|----------|-------------|
| `<ENV>` | Environment name from `metaphor.deploy.yaml` (must have `host:`) |

### Options

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--tag` | string | git short SHA | Image tag to build/push |
| `--skip-build` | bool | `false` | Don't build/push; assume images already exist in the registry under `<tag>` |
| `--skip-migrate` | bool | `false` | Skip the post-rollout migration step |
| `--skip-env-update` | bool | `false` | Don't rewrite `*_TAG=…` entries in the local env file |
| `--dry-run` | bool | `false` | Print every command without executing |
| `-y`, `--yes` | bool | `false` | Skip the confirmation prompt for environments with `require_confirm: true` |

### What `push` does, step by step

1. **Resolve tag.** Use `--tag` if given, otherwise `git rev-parse --short HEAD` from the workspace root.
2. **Confirm** if `environments.<env>.require_confirm` is true and `--yes` was not passed.
3. **Build & push** each image under `environments.<env>.images`:
   ```
   docker buildx build --platform linux/amd64 \
     -t <registry>/<image_name>:<tag> \
     [--build-arg KEY=VALUE …] \
     [-f <dockerfile>] \
     --push \
     <context>
   ```
   `<image_name>` defaults to the map key. Skipped when `--skip-build`.
4. **Update env file** locally. For each image with a `tag_env` field, replace or append `<tag_env>=<tag>` in the local env file. Skipped when `--skip-env-update`.
5. **`scp`** the local env file to `<ssh_user>@<host>:<deploy_dir>/<remote_env_file>`.
6. **`ssh`** to host and run `docker compose -f <compose> --env-file <env_file> pull`.
7. **`ssh`** again and run `docker compose … up -d`.
8. **Migrate** by running `docker compose run --rm migrations sh -lc "<migrate_command>"` on the remote host (skipped when `--skip-migrate`). The `<migrate_command>` defaults to `metaphor migration run-all` and can be overridden by `defaults.migrate_command`.

### Examples

Standard release of HEAD to UAT:

```sh
metaphor-dev deploy push uat
```

Dry run — show every command but don't execute:

```sh
metaphor-dev deploy push prod --dry-run
```

Reuse images already in the registry (e.g. promotion from UAT to prod):

```sh
metaphor-dev deploy push prod --tag abc1234 --skip-build
```

Push without running migrations (e.g. when the change is image-only):

```sh
metaphor-dev deploy push uat --skip-migrate
```

Non-interactive prod release (CI):

```sh
metaphor-dev deploy push prod --tag $GITHUB_SHA --yes
```

---

## deploy rollback

Roll a remote environment back to a tag already in the registry.

### Synopsis

```
metaphor-dev deploy rollback <ENV> --to <TAG> [-y]
```

### Arguments

| Argument | Description |
|----------|-------------|
| `<ENV>` | Environment name |

### Options

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--to` | string | required | Tag to roll back to |
| `-y`, `--yes` | bool | `false` | Skip confirmation for `require_confirm` envs |

### Behavior

Identical to `push` from step 4 onward — there is no implicit "previous tag" memory; the operator must specify `--to`.

### Examples

```sh
metaphor-dev deploy rollback prod --to abc1234 --yes
```

---

## deploy status

`docker compose ps` against the remote env.

### Synopsis

```
metaphor-dev deploy status <ENV>
```

### Examples

```sh
metaphor-dev deploy status uat
```

---

## deploy logs

`docker compose logs` against the remote env.

### Synopsis

```
metaphor-dev deploy logs <ENV> [OPTIONS]
```

### Options

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--service` | string | none | Limit to a single service |
| `--tail` | string | `200` | Lines from the end |
| `-f`, `--follow` | bool | `false` | Stream new lines |

### Examples

Tail the API service:

```sh
metaphor-dev deploy logs prod --service bersihir-service --follow
```

---

## deploy migrate

Run database migrations against the remote environment.

### Synopsis

```
metaphor-dev deploy migrate <ENV> [--dry-run]
```

### Behavior

Runs `docker compose run --rm migrations sh -lc "<migrate_command>"` on the remote host. The `<migrate_command>` is taken from `defaults.migrate_command` and falls back to `metaphor migration run-all`.

This assumes the compose file declares a `migrations` service (typically a one-shot container that shares the application image and has database access).

### Examples

```sh
metaphor-dev deploy migrate uat
metaphor-dev deploy migrate prod --dry-run
```

> If your migration workflow uses an SSH tunnel from the operator's machine instead (e.g. `ssh -L 5433:postgres:5432 deploy@host` + a local `metaphor migration run-all`), set `defaults.migrate_command` to a wrapper script that performs the tunnel-based flow.

---

## deploy exec

Delegate to the workspace's infra project (`./deploy.sh` or `make deploy`). This is the **legacy** workflow inherited from the original native `metaphor deploy` command — kept here so all deploy-shaped verbs live in one place. Use it when your repo is structured around an `infra` project that owns its own deployment scripts, rather than around `metaphor.deploy.yaml`.

### Synopsis

```
metaphor-dev deploy exec [--infra <NAME>] [-- ARGS...]
```

### Options

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--infra` | string | sole `infra` project | Select a specific infra project when multiple are registered |
| trailing args | list | none | Forwarded verbatim to the chosen deploy command |

### Behavior

1. Walks up from the current directory to find `metaphor.yaml`.
2. Picks the project with `type: infra` (errors if zero or ambiguous; use `--infra <name>` to disambiguate).
3. From that project's directory, runs the **first** of:
   - `./deploy.sh <args>` (if executable)
   - `make deploy <args>` (if a `Makefile` is present)
4. Exits non-zero if neither exists or the chosen command fails.

Unlike the other `deploy` subcommands, `exec` does **not** read `metaphor.deploy.yaml` and does **not** invoke docker, ssh, or scp. It is purely a shell-out to the infra project.

### Examples

Run the sole infra project's deploy script:

```sh
metaphor-dev deploy exec
```

Pass arguments through:

```sh
metaphor-dev deploy exec -- ENVIRONMENT=prod --dry-run
```

Disambiguate when multiple infra projects are registered:

```sh
metaphor-dev deploy exec --infra infra-prod -- --tag $GIT_SHA
```

### When to use `exec` vs `push`

| Use `exec` when… | Use `push` when… |
|------------------|------------------|
| Deploy logic lives in `infra/deploy.sh` or a `Makefile` | Deploy logic is `docker buildx → scp env → ssh + docker compose` |
| You're migrating an existing project that already has its own deploy script | You're starting fresh and want a declarative `metaphor.deploy.yaml` |
| The script does things outside docker (Terraform, Ansible, k8s manifests) | The target is a single host running docker compose |

---

## Configuration

`deploy` commands read [`metaphor.deploy.yaml`](../reference/configuration.md#metaphordeployyaml) at the workspace root.

| Field | Source | Purpose |
|-------|--------|---------|
| `environments.<env>.host` | per-env | SSH host (required — its presence marks the env as remote) |
| `environments.<env>.ssh_user` | per-env, falls back to `defaults.ssh_user` | SSH user; concatenated as `user@host` |
| `environments.<env>.deploy_dir` | per-env, falls back to `defaults.deploy_dir` | Working directory on the remote host |
| `environments.<env>.compose_file` | per-env, falls back to `defaults.compose_file` | Compose file path **relative to `deploy_dir`** on the remote host |
| `environments.<env>.env_file` | per-env, falls back to `.env.<env>` | Env file path; resolved against the workspace root locally and `deploy_dir` remotely |
| `environments.<env>.registry` | per-env, falls back to `defaults.registry`, then per-image override | Container registry prefix used for pushed image tags |
| `environments.<env>.require_confirm` | per-env | Prompt before push/rollback unless `--yes` |
| `environments.<env>.images.<key>` | per-env | Image build spec; see [Configuration Reference](../reference/configuration.md#metaphordeployyaml) |
| `defaults.migrate_command` | top-level | Command run by `deploy migrate` (default `metaphor migration run-all`) |

### Image spec fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `context` | string | yes | Build context path relative to workspace root |
| `dockerfile` | string | no | Dockerfile path relative to `context` |
| `name` | string | no | Image name (defaults to the map key) |
| `registry` | string | no | Per-image registry override |
| `tag_env` | string | no | Env-file variable that tracks this image's tag (e.g. `SERVICE_TAG`) |
| `build_args` | map<string,string> | no | `--build-arg` pairs forwarded to `docker buildx build` |
| `push` | bool | no | Push after build (default `true` for images under remote envs) |

---

## Examples: a typical release

```sh
# 1. Local build + UAT rollout from current branch
metaphor-dev deploy push uat

# 2. Verify
metaphor-dev deploy status uat
metaphor-dev deploy logs uat --service bersihir-service --tail 500

# 3. Promote to prod with the same tag (no rebuild)
metaphor-dev deploy push prod --tag $(git rev-parse --short HEAD) --skip-build --yes

# 4. If something goes wrong, roll prod back to the previous SHA
metaphor-dev deploy rollback prod --to <previous-sha> --yes
```

---

## Troubleshooting

### `environment 'X' is local (no host: set)`

The named environment has no `host:` field. Either add one in `metaphor.deploy.yaml` or use [`metaphor-dev docker`](docker.md) for local operations.

### `git rev-parse failed`

Either the workspace isn't a git repository, or HEAD has no commit. Pass `--tag <value>` to skip the git lookup entirely.

### `deploy_dir not set for remote environment`

Set `deploy_dir` either at `environments.<env>.deploy_dir` or at `defaults.deploy_dir`.

### `failed to spawn \`scp\` / \`ssh\``

Install OpenSSH client tools and ensure they're on `PATH`. Verify you can reach the host manually first:

```sh
ssh deploy@host 'docker compose version'
```

### Permission denied on the remote host

`deploy push` runs `docker compose` over SSH, which requires the SSH user to be in the `docker` group on the remote host (or to use `sudo`, which this plugin does not invoke). Add the user to the group with `sudo usermod -aG docker <user>` and reconnect.

### Push succeeded but rollout didn't take effect

Verify the env file was transported:

```sh
ssh deploy@host 'cat /srv/app/.env.uat | grep _TAG='
```

If `*_TAG` values still point at the previous SHA, either `--skip-env-update` was passed in error, or the local env file did not have entries for those variables. Add `<tag_env>=` lines to the local env file (any value is fine — they'll be overwritten on next push) and try again.

---

## See Also

- [docker](docker.md) — Local docker compose lifecycle that shares `metaphor.deploy.yaml`
- [Configuration Reference](../reference/configuration.md#metaphordeployyaml) — Full schema for `metaphor.deploy.yaml`
- [CI Integration Guide](../guides/ci-integration.md) — Running `deploy push` from CI
