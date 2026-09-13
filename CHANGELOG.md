# Changelog

All notable changes to `metaphor-plugin-dev` are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.1] - 2026-09-13

### Fixed

- `metaphor test` now exits non-zero when tests fail. It printed "Some tests
  failed" and returned success, so every script, hook and CI job gating on the
  command was gating on nothing. Same defect the lint gates carried in 0.2.0,
  in the same crate.
- `metaphor dev serve` reports a crashed application instead of swallowing it,
  while still treating Ctrl+C as the ordinary way to stop a dev server. The two
  were folded together before: any non-zero exit printed a line and returned
  success. A process ended by a signal carries no exit code, which is the
  signal used to tell them apart.

## [0.2.0] - 2026-09-13

### Fixed

- `metaphor lint` now exits non-zero when a gate finds something. Every check —
  clippy, rustfmt, the compilation check, the security audit and the aggregate
  `lint all` — printed a red cross and returned success, so any script or CI job
  built on the command reported a green lint over broken code. Each gate now
  reports a pass/fail/skip outcome that reaches the process exit code, and
  `lint all` names every gate that failed in its verdict.
- `lint all` no longer claims "All quality checks passed" when checks failed. Its
  pass flag only flipped on an error return that the gates never produced, so the
  celebratory summary printed unconditionally.
- A missing `cargo-audit` or `cargo-outdated` is detected correctly. The probe
  tested whether spawning cargo itself failed, which it does not when only the
  subcommand is absent, so the missing tool fell through to the real invocation
  and its "no such subcommand" exit was reported as if vulnerabilities had been
  found. A missing tool is now reported as a skip, and the dead auto-install
  branch that could never be reached has been removed — a lint command does not
  install software behind the caller's back.
- A `cargo-audit` run that cannot read its advisory database is reported as a
  skipped scan instead of as detected vulnerabilities, so nobody chases CVEs
  that were never read.

### Added

- `--require-tools` on `metaphor lint check` and `metaphor lint all`, which turns
  a skipped gate into a failure. Without it a pipeline can go green because a
  linter was missing from the image; with it, the gate must actually have run.
- `--strict` on `metaphor lint all` now also promotes a security finding from
  advisory to blocking. The audit stays advisory by default so an advisory
  published upstream today does not break a build that changed nothing.

## [0.1.10] - 2026-09-10

### Added

- `metaphor chaos` — a thin forwarder to the workspace's fault-injection kit
  (`deployment/chaos/run.sh`), so the post-change chaos gate has a first-class
  CLI entry: `metaphor chaos list`, `metaphor chaos --dry-run all`,
  `metaphor chaos <experiment>`, `metaphor chaos --full all`. Resolves the
  workspace root from `metaphor.yaml`, refuses loudly when the workspace ships
  no kit (pointing at the metaphor-workspace template), forwards every
  argument verbatim, and exits with the driver's exit code so CI and agents
  can gate on it. No fault logic lives in Rust — the bash driver stays the
  single source of truth for the catalog and verdict semantics.

## [0.1.9] - 2026-08-16

### Added

- `lint check|all` now runs `metaphor-schema validate-workspace` **before**
  clippy, so the ADR-0014 company-fence sweep is machine-checked on every
  lint — a module without a `company_fence:` declaration fails the lint, not
  just a distant validate. Inside a module directory the gate scopes to that
  module alone (an unswept sibling elsewhere in the workspace never fails
  your local lint).

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
