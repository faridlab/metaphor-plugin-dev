# metaphor-dev dev

Development workflow commands for the Metaphor framework. This command group provides subcommands for starting development servers, running tests, building the project, and managing database operations.

---

## Subcommands

| Subcommand | Description |
|------------|-------------|
| [`dev serve`](#dev-serve) | Start development servers (gRPC + REST + CLI) |
| [`dev test`](#dev-test) | Run all tests (unit + integration + E2E) |
| [`dev build`](#dev-build) | Build the entire project |
| [`dev db`](#dev-db) | Database operations (migrate, create, reset) |

---

## dev serve

Start development servers including gRPC, REST, and CLI services.

### Synopsis

```
metaphor-dev dev serve [OPTIONS]
```

### Description

Starts the Metaphor development environment by launching gRPC and REST services. The command loads configuration from `apps/metaphor/config/application.yml` and applies an environment overlay from `apps/metaphor/config/application-{APP_ENV}.yml` when available. If the config file is not found, built-in defaults are used.

By default, all services are started in local mode using `cargo run`. You can restrict which services start with `--grpc-only` or `--rest-only`, and choose the orchestration method with `--docker` or `--local`.

### Options

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--grpc-only` | bool | `false` | Start only gRPC services |
| `--rest-only` | bool | `false` | Start only REST services via Envoy |
| `--port` | u16 | `3000` | Main listening port |
| `--docker` | bool | `false` | Use Docker Compose to orchestrate services |
| `--local` | bool | `false` | Run with `cargo run` directly (default if neither `--docker` nor `--local` is specified) |

### Mutual Exclusion Rules

- `--grpc-only` and `--rest-only` cannot be used together.
- `--docker` and `--local` cannot be used together.

### Default Services and Ports

| Service | Name | REST Port | gRPC Port | Description |
|---------|------|-----------|-----------|-------------|
| metaphor | Metaphor API Gateway | 3000 | 50051 | Main API Gateway and orchestrator |
| sapiens | Sapiens User Management | 3003 | 50053 | User management and authentication |
| postman | Postman Email Service | 3002 | 50052 | Email sending and notification service |
| bucket | Bucket File Storage | 3004 | 50054 | File storage and media management |

### Available Endpoints

Once the server is running, the following endpoints are available:

| Endpoint | URL |
|----------|-----|
| REST API | `http://localhost:{port}/api/v1` |
| gRPC Services | `localhost:50051` |
| Health Check | `http://localhost:{port}/health` |
| MongoDB | `mongodb://root:password@localhost:27017` |
| PostgreSQL | `postgresql://root:password@localhost:5432` |

### Behavior Details

- **Local mode** (default): Runs `cargo run --bin metaphor-app` in the `apps/metaphor` directory. Sets `APP_ENV=development` and `DATABASE_URL` environment variables automatically.
- **Docker mode**: Checks for the `docker-compose` binary and a `docker-compose.yml` file, then starts services via `docker-compose up -d`. After startup, performs HTTP health checks on all enabled services with a 5-second timeout. The health check endpoint is `/health` for every service.

### Examples

Start all services in local mode on the default port:

```sh
metaphor-dev dev serve
```

Start only gRPC services on a custom port:

```sh
metaphor-dev dev serve --grpc-only --port 4000
```

Start all services using Docker Compose:

```sh
metaphor-dev dev serve --docker
```

### Notes

- If neither `--docker` nor `--local` is specified, local mode is used by default.
- The health check timeout is 5 seconds per service in Docker mode.
- Module availability (sapiens, postman, bucket) is controlled by the `modules` section of the configuration file.

---

## dev test

Run all tests including unit, integration, and end-to-end tests.

### Synopsis

```
metaphor-dev dev test [OPTIONS]
```

### Description

Executes the project test suite. By default, all available test types are run. Use the filter flags to restrict which test types execute.

### Options

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--unit-only` | bool | `false` | Run only unit tests |
| `--integration-only` | bool | `false` | Run only integration tests |
| `--e2e-only` | bool | `false` | Run only end-to-end tests |
| `--coverage` | bool | `false` | Enable code coverage |

### Examples

Run the full test suite:

```sh
metaphor-dev dev test
```

Run only unit tests with coverage enabled:

```sh
metaphor-dev dev test --unit-only --coverage
```

Run only integration tests:

```sh
metaphor-dev dev test --integration-only
```

### Notes

- You cannot specify all three of `--unit-only`, `--integration-only`, and `--e2e-only` at the same time.
- Unit tests are executed via `cargo test --lib`.
- Integration and E2E tests are not yet implemented (Priority 2).

---

## dev build

Build the entire project.

### Synopsis

```
metaphor-dev dev build [OPTIONS]
```

### Description

Compiles the project using Cargo. Supports release-optimized builds and an optional post-build test run.

### Options

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--release` | bool | `false` | Build in release mode |
| `--test` | bool | `false` | Run tests after a successful build |

### Examples

Build the project in debug mode:

```sh
metaphor-dev dev build
```

Build an optimized release binary and run tests afterward:

```sh
metaphor-dev dev build --release --test
```

### Notes

- Debug mode is used by default. Pass `--release` to produce an optimized build.
- When `--test` is specified, tests run only if the build succeeds.

---

## dev db

Database operations including migrations, creation, and reset.

### Synopsis

```
metaphor-dev dev db <SUBCOMMAND> [OPTIONS]
```

### Subcommands

#### dev db migrate

Run database migrations.

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--version` | i64 | none | Target a specific migration version |

**Examples:**

Run all pending migrations:

```sh
metaphor-dev dev db migrate
```

Migrate to a specific version:

```sh
metaphor-dev dev db migrate --version 20260101120000
```

#### dev db create

Create a new migration file.

| Argument | Type | Required | Description |
|----------|------|----------|-------------|
| `name` | string | yes | Name of the migration |

**Examples:**

```sh
metaphor-dev dev db create add_users_table
```

#### dev db reset

Reset the database by dropping and recreating it.

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--force` | bool | `false` | Skip the confirmation prompt |

**Examples:**

Reset the database with a confirmation prompt:

```sh
metaphor-dev dev db reset
```

Force reset without confirmation:

```sh
metaphor-dev dev db reset --force
```

### Notes

- Database operations are marked as Priority 2.1 and are not yet fully implemented.

---

## Configuration

The `dev` commands load configuration from `apps/metaphor/config/application.yml`. The `DevConfig` struct contains the following sections:

| Section | Description | Defaults |
|---------|-------------|----------|
| `server` | Host and port settings | `0.0.0.0:3000` |
| `modules` | Which modules are enabled (sapiens, postman, bucket) | Varies by environment |
| `services` | Per-service configuration | See default services table above |

### Environment Overlays

Configuration supports environment-specific overrides. Set the `APP_ENV` environment variable and the system will also load `apps/metaphor/config/application-{APP_ENV}.yml`, merging it on top of the base configuration. If no config file is found, the built-in defaults are used.

---

## Troubleshooting

### Server fails to start in local mode

- Verify that `apps/metaphor/` exists and contains a valid Cargo project with a `metaphor-app` binary target.
- Ensure the required environment variables (`APP_ENV`, `DATABASE_URL`) are set or that the defaults are acceptable.

### Docker mode fails immediately

- Confirm that `docker-compose` (or `docker compose`) is installed and available on your `PATH`.
- Verify that a `docker-compose.yml` file exists in the expected location.

### Health checks time out after Docker startup

- The default timeout is 5 seconds. If services take longer to boot, wait and retry manually: `curl http://localhost:3000/health`.
- Check container logs with `docker-compose logs` to identify startup errors.

### Tests fail to run

- Ensure the project compiles successfully with `metaphor-dev dev build` before running tests.
- Integration and E2E tests are not yet implemented; only `--unit-only` produces results currently.

### Database commands do nothing

- Database operations are not yet fully implemented (Priority 2.1). Check for updates in future releases.

---

## See Also

- [Getting Started Guide](../guides/getting-started.md)
- [Development Workflow Guide](../guides/development-workflow.md)
