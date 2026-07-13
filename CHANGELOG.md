# Changelog

All notable changes to `metaphor-plugin-dev` are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.8] - 2026-07-13

### Changed

- `deploy migrate <env>` now gates on a **typed env-name confirmation** for
  `require_confirm` environments — the operator must type the exact env name to
  proceed, so an irreversible prod migration can't run on a stray keypress. Added
  `--yes` to bypass the prompt in CI; `--dry-run` still never executes. Migrations
  run as part of `deploy push` reuse push's own confirmation and are not prompted
  twice.

### Documentation

- Documented the `deploy migrate` confirmation gate and `--yes` flag in
  `docs/commands/deploy.md`.

## [0.1.7] - 2026-06-30

### Added

- `deploy service <env> <svc> <tag>` — deploy a single pre-built service from the
  registry (no build, no migrate). Bumps just that service's `*_TAG`, scp's the env
  file, then pulls + `up -d` + `ps` only that service on the remote. Records the
  deploy in history. History-aware successor to the legacy `deploy-service.sh`.
- `deploy bump <env> --service <svc> --tag <tag>` — bump a service's `*_TAG` in the
  LOCAL env file only (no SSH, no deploy), staging the change for review/commit
  before deploying. Includes no-op detection (successor to `bump-prod-tag.sh`).
- `deploy preflight <env>` — validate local prod env files before a push (no SSH):
  per-service contract check against each image's `.env.prod.example`, plus a
  `docker compose config` interpolation check for unresolved `${VAR:?}` references.

### Documentation

- Documented `deploy service`, `deploy bump`, and `deploy preflight` in
  `docs/commands/deploy.md` and the README command table.

## [0.1.6] - 2026-04-25

### Added

- `deploy history <env>` — show deployment history (text/JSON, local or `--remote`).
- History-aware `deploy rollback` (`--steps N` / `--to TAG`) reading per-env JSONL.
- Deploy history module: JSONL records, env-file snapshots, and remote mirroring.
  Each `deploy push` now records success/failure for audit and rollback.

## [0.1.5] - 2026-04-25

### Added

- Deploy history module groundwork.

## [0.1.4] - 2026-04-25

### Added

- `docker` — local docker compose lifecycle command sharing `metaphor.deploy.yaml`.
- `deploy` — remote deploy `push`, `rollback`, `status`, `logs`, `migrate`, `exec`.
- `metaphor.deploy.yaml` config loader and wiring into the CLI dispatcher.

### Documentation

- Documented `docker`, `deploy`, and the `metaphor.deploy.yaml` schema.

## [0.1.3] - 2026

### Added

- Multi-app workspace project resolver; `config` and `dev` resolve paths through it.

### Fixed

- Generalized the `dev` setup-instruction path.

## [0.1.2] - 2026

### Added

- Project resolver for multi-app workspaces.

## [0.1.0] - 2026

### Added

- Initial release: `dev`, `lint`, `test`, `docs`, `config`, and `jobs` commands.
- Command reference docs, guides (getting started, workflow, CI), and reference docs.
- GitHub Actions release workflow.
