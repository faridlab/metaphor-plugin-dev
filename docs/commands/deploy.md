# metaphor-dev deploy

Remote deployment lifecycle for environments declared in [`metaphor.deploy.yaml`](../reference/configuration.md#metaphordeployyaml). Operates on environments that have a `host:` field — for purely local stacks use [`docker`](docker.md) instead.

The model is intentionally thin: each command is a deterministic combination of `docker buildx`, `scp`, `ssh`, and `docker compose`. There is no bespoke orchestration layer or state store.

> **Invocation:** examples below use the standalone plugin form `metaphor-dev deploy …`. When invoked via the core CLI, drop the `-dev` suffix: `metaphor deploy …`. Both routes are equivalent.

---

## Subcommands

| Subcommand | Description |
|------------|-------------|
| [`deploy push`](#deploy-push) | Build, push to registry, and roll out a release |
| [`deploy service`](#deploy-service) | Deploy ONE pre-built service from the registry (no build/migrate) |
| [`deploy bump`](#deploy-bump) | Bump a service's `*_TAG` in the LOCAL env file only (no SSH, no deploy) |
| [`deploy preflight`](#deploy-preflight) | Validate local prod env files before a push (no SSH) |
| [`deploy rollback`](#deploy-rollback) | Roll back to a previous deploy (history-aware) |
| [`deploy history`](#deploy-history) | Show deployment history for an environment |
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
9. **Record history** by appending a JSONL entry to `deployment/history/<env>.jsonl`, snapshotting the env file under `deployment/history/snapshots/`, and mirroring both to `<deploy_dir>/history/` on the remote host. A failed push still gets recorded (with `status: failed` and a short error) so [`deploy rollback`](#deploy-rollback) can step over it.

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

## deploy service

Deploy a **single** pre-built service from the registry, without building or migrating. Use it when CI (or a prior `deploy push`) has already pushed the image and you only need to roll that one service forward on the remote host. The image must already exist in the registry under `<tag>`.

This is the history-aware successor to the legacy `deploy-service.sh`: unlike that script, every `deploy service` is recorded in [`deploy history`](#deploy-history).

### Synopsis

```
metaphor-dev deploy service <ENV> <SERVICE> <TAG> [OPTIONS]
```

### Arguments

| Argument | Description |
|----------|-------------|
| `<ENV>` | Environment name from `metaphor.deploy.yaml` (must have `host:`) |
| `<SERVICE>` | Service to deploy — a key under the env's `images` |
| `<TAG>` | Image tag already present in the registry (e.g. `v0.1.2`) |

### Options

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--dry-run` | bool | `false` | Print every command without executing |
| `-y`, `--yes` | bool | `false` | Skip the confirmation prompt for environments with `require_confirm: true` |

### What `service` does, step by step

1. **Validate** that `<SERVICE>` is a key under `environments.<env>.images` (errors with the available names if not).
2. **Confirm** if `require_confirm: true` and `--yes` was not passed.
3. **Bump** only this service's `<tag_env>=<tag>` in the local env file. Errors if the selected image has no `tag_env` — an explicit selection means you asked for this service specifically.
4. **Snapshot** the env file for audit/rollback.
5. **`scp`** the local env file to the remote host.
6. **Pull → up → ps**, scoped to just this service:
   ```
   docker compose … pull <service>
   docker compose … up -d <service>
   docker compose … ps  <service>
   ```
7. **Record history** — appends a `push` record (per-image tag map = just this service) to `deployment/history/<env>.jsonl` and mirrors it to the remote on success. A failure is still recorded with `status: failed`.

`--dry-run` previews every command and writes nothing — no env-file edit, no snapshot, no history record.

### Examples

Roll the API service forward to a tag CI just pushed:

```sh
metaphor-dev deploy service prod api v0.1.2 --yes
```

Preview without touching anything:

```sh
metaphor-dev deploy service prod api v0.1.2 --dry-run
```

---

## deploy bump

Bump a service's `*_TAG` in the **local** env file only — no build, no SSH, no deploy. It stages the tag change so you can review the diff and commit before deploying. Pairs with a later [`deploy service`](#deploy-service) or [`deploy push`](#deploy-push).

This is the successor to the legacy `bump-prod-tag.sh`, including its no-op detection: if the variable is already at the requested tag, it prints "No change" and exits.

### Synopsis

```
metaphor-dev deploy bump <ENV> --service <SERVICE> --tag <TAG>
```

### Arguments

| Argument | Description |
|----------|-------------|
| `<ENV>` | Environment name from `metaphor.deploy.yaml` |

### Options

| Flag | Type | Required | Description |
|------|------|----------|-------------|
| `--service` | string | yes | Service whose tag var to bump — a key under the env's `images` |
| `--tag` | string | yes | New tag value (e.g. `v0.1.2`) |

### Behavior

1. Resolve the service's `tag_env` (errors if it has none).
2. If the env file already has `<tag_env>=<tag>`, report "No change" and exit successfully.
3. Otherwise rewrite `<tag_env>=<tag>` in the local env file and print the next step.

Unlike `deploy service`, `bump` never touches the remote host and writes no history record — it is a purely local edit. It works on local-only environments too (it does not require `host:`).

### Examples

Stage a tag bump for review:

```sh
metaphor-dev deploy bump prod --service api --tag v0.1.2
# → review the diff, commit, then:
metaphor-dev deploy service prod api v0.1.2
```

---

## deploy preflight

Validate the local prod env files **before** a push. Local-only — it makes no SSH connection. Run it as a fast gate in CI or before `deploy push` to catch missing/placeholder secrets before they reach the remote host.

### Synopsis

```
metaphor-dev deploy preflight <ENV>
```

### Arguments

| Argument | Description |
|----------|-------------|
| `<ENV>` | Environment name from `metaphor.deploy.yaml` |

### Behavior

Two layers, both must pass:

1. **Per-service contract check.** For each image whose `<context>/.env.prod.example` exists, every `^[A-Z_]+=` variable declared there must be present, non-empty and non-placeholder in `<context>/.env.prod`. Images without a `.env.prod.example` (e.g. webapps) are auto-skipped. A value is a placeholder if it matches `CHANGE_ME`, `TODO`, `REPLACE_ME`, `FILL_…`, `XXX…`, or a `<…>` angle-bracket token.
2. **Compose interpolation check.** Runs `docker compose -f <compose> --env-file <env> config` and fails if any `${VAR:?}` reference is unresolved.

Exits non-zero on the first failing layer, listing each missing or placeholder variable.

### Examples

Gate a release:

```sh
metaphor-dev deploy preflight prod && metaphor-dev deploy push prod
```

### Errors

- `<service>: runtime env file missing: <path>` — the image's `.env.prod` does not exist next to its `.env.prod.example`.
- `[missing] <VAR>` / `[placeholder] <VAR>` — a contract variable is absent or still set to a placeholder.
- `compose validation failed: unresolved ${VAR:?} refs in <env>` — `docker compose config` could not resolve a required interpolation.

---

## deploy rollback

Roll a remote environment back to a previous deploy. Reads `deployment/history/<env>.jsonl` to figure out where to roll back to — no need to look up SHAs by hand. Pass `--to <tag>` for an explicit tag override.

### Synopsis

```
metaphor-dev deploy rollback <ENV> [--steps N | --to TAG] [-y]
```

### Arguments

| Argument | Description |
|----------|-------------|
| `<ENV>` | Environment name |

### Options

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--steps` | int | `1` | Number of successful pushes to step back from current. `1` = the deploy before this one. Mutually exclusive with `--to`. |
| `--to` | string | none | Roll back to this exact tag. Bypasses history. Mutually exclusive with `--steps`. |
| `-y`, `--yes` | bool | `false` | Skip confirmation for `require_confirm` envs |

### Behavior

1. Read `deployment/history/<env>.jsonl`.
2. Resolve the target:
   - `--steps N`: walk back `N` successful pushes from the most recent one. Failed pushes and rollbacks are skipped so step counts always refer to "deploys that actually shipped".
   - `--to <tag>`: use this exact tag for every image with a `tag_env`; assume the registry has it.
3. Confirm if `require_confirm: true` and `--yes` was not passed.
4. Update the local env file with the target tags, `scp` to the remote host, `docker compose pull && up -d`.
5. Append a `rollback` record to `deployment/history/<env>.jsonl` and mirror it to the remote host.

### Examples

Roll back to the previous successful deploy (most common):

```sh
metaphor-dev deploy rollback prod
```

Roll back two pushes:

```sh
metaphor-dev deploy rollback prod --steps 2
```

Explicit tag (e.g. promoting a known-good UAT image straight to prod):

```sh
metaphor-dev deploy rollback prod --to abc1234 --yes
```

### Errors

- `no deployment/history/<env>.jsonl yet` — there's no recorded deploy to step back from. Use `--to <tag>` instead.
- `step N resolves to the currently-deployed tag` — the step count points at the same tag that's already live; nothing to do.

### Notes

- Rollback is **image-only**. Database migrations are not auto-reverted; schema rollbacks must be forward-fix migrations.
- If the historical record had per-image tags (e.g. a frontend was pushed independently), they're restored individually.

---

## deploy history

Show recent deployments for an environment.

### Synopsis

```
metaphor-dev deploy history <ENV> [OPTIONS]
```

### Options

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--limit` | int | `20` | Most recent N records to show |
| `--remote` | bool | `false` | Read from `<deploy_dir>/history/<env>.jsonl` over SSH instead of the local workspace file |
| `--json` | bool | `false` | Emit raw JSON array (newest first) for scripting |

### Output

Default text view (newest first):

```
TIMESTAMP (UTC)       ACTION    TAG         OK   DEPLOYER
2026-04-25 14:02:00   push      abc1234     ✓    farid@laptop
2026-04-25 11:30:00   rollback  def5678     ✓    farid@laptop
2026-04-25 09:15:00   push      def5678     ✗    farid@laptop
    error: ssh exited 255: Connection refused
2026-04-24 16:45:00   push      def5678     ✓    farid@laptop
```

### Storage

Records are appended JSONL at `deployment/history/<env>.jsonl` in the workspace and (best-effort) mirrored to `<deploy_dir>/history/<env>.jsonl` on the remote host. Each record carries: timestamp, action (`push`/`rollback`), status (`success`/`failed`), tag, per-image tag map, deployer (`$USER@$HOSTNAME`), optional snapshot reference, optional rollback source tag, and a short error if the action failed.

Snapshots of the env file used for each successful deploy are written to `deployment/history/snapshots/.env.<env>.<timestamp>-<sha>` for audit / future replay. They are referenced by basename in the JSONL but not auto-pruned — the file is intended to be a permanent record.

History is never auto-pruned by the CLI. The local file should be committed to git so it survives laptop loss; the remote mirror is for ops convenience.

### Examples

Last 20 deploys (workspace history):

```sh
metaphor-dev deploy history prod
```

Last 5, in JSON for scripting:

```sh
metaphor-dev deploy history prod --limit 5 --json
```

Read directly from the production host (handy when on-call without checkout):

```sh
metaphor-dev deploy history prod --remote
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
metaphor-dev deploy migrate <ENV> [--dry-run] [--yes]
```

### Options

| Flag | Type | Default | Meaning |
|------|------|---------|---------|
| `--dry-run` | bool | `false` | Print the SSH + compose migrate command without executing (never prompts) |
| `-y`, `--yes` | bool | `false` | Skip the typed-env-name confirmation for `require_confirm` envs (required in CI) |

### Behavior

Runs `docker compose run --rm migrations sh -lc "<migrate_command>"` on the remote host. The `<migrate_command>` is taken from `defaults.migrate_command` and falls back to `metaphor migration run-all`.

This assumes the compose file declares a `migrations` service (typically a one-shot container that shares the application image and has database access).

**Confirmation gate.** Migrations against an environment with `require_confirm: true` (e.g. prod) are irreversible schema/data changes, so `deploy migrate` requires the operator to **type the exact env name** to proceed — a plain keypress is not enough. Pass `--yes` to bypass this (CI), or `--dry-run` which never executes anything. A migration triggered as part of `deploy push` reuses `push`'s own confirmation and is not prompted a second time.

### Examples

```sh
metaphor-dev deploy migrate uat
metaphor-dev deploy migrate prod --dry-run
metaphor-dev deploy migrate prod            # prompts: type "prod" to proceed
metaphor-dev deploy migrate prod --yes      # CI: no prompt
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
| `environments.<env>.require_confirm` | per-env | Prompt before push/rollback/service (and type-env-name confirm before migrate) unless `--yes` |
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
