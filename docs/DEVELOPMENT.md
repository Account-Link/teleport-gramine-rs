# Development Guide

This guide provides essential information for developers working on the Teleport project.

## Environment Setup

### Prerequisites

- Install the latest version of Docker
- Linux distribution with SGX support (e.g., Ubuntu 20.04 LTS)
- SGX driver installed (follow manufacturer instructions)

### Configuration

The project uses a layered configuration system, combining default settings, environment-specific overrides, and secret management. The configuration is loaded from multiple sources in the following order:

1. [config.toml](/config.toml): Contains both default settings and environment-specific configurations.
   - The `[default]` section defines base settings for all environments. It also defines the settings for the development environment. The backend will by default runs on development mode.
   - Profile-specific sections (e.g., `[staging]`, `[production]`) override default settings for each environment.

2. Environment variables:
   - The `APP_ENV` environment variable determines which profile to use (development, staging, or production).
   - Environment variables prefixed with `APP_` can override any configuration value.

3. `private.env`: Contains sensitive information and secrets.
   - This file should be created by copying [private.env.example](/private.env.example) and filling in the required values.
   - It is loaded using the `dotenv` crate or read from the environment variables and should not be committed to version control.

The configuration is managed by the `Config` struct in [src/config.rs](/src/config.rs), which uses the `figment` crate for flexible configuration management.

To use a specific environment configuration:

1. Set the `APP_ENV` environment variable to "development", "staging", or "production".
2. Ensure the corresponding section (except for development which is the default) exists in [config.toml](/config.toml).
3. Any environment-specific overrides will be applied on top of the default configuration.

Remember to keep sensitive information in `private.env` and never commit this file to version control. Use [private.env.example](/private.env.example) as a template for the required secret variables.

## Docker Configuration

Our project uses Docker for containerization. The main configuration files are:

- [Dockerfile](/Dockerfile): Defines how the project image is built. It includes steps for setting up the environment, installing dependencies, and building the project.
- [docker-compose.yml](/docker-compose.yml): Defines the service setup, including environment variables, volume mounts, and device mappings for SGX support.

Refer to these files directly for the most up-to-date information on our Docker setup.

## Development Mode

To facilitate easier iteration on the non-TEE components of the application, we provide a development mode that allows you to run the server without TEE, Gramine, TLS, or Docker.

To start the server in development mode, simply execute `cargo run` from the root of the repository. This command will launch the server on `http://localhost:3000`.

It is highly recommended to enable logging during development by running:

```bash
RUST_LOG=info cargo run
```

## Building and Running TEE server

```bash
# Build the Docker image
docker compose build

# Start the server
docker compose run --rm teleport "make start-gramine-server"
```

## Debugging TEE server

Note on SGX settings:

- The Dockerfile builds both SGX and non-SGX versions of the application.
- The docker-compose.yml file sets `SGX=1` as the default environment variable.
- For debugging, we override this setting with `SGX=` to use the non-SGX version.

To run the project in debug mode:

```bash
docker compose run --rm teleport "DEBUG=1 SGX= make start-gramine-server"
```

This command:

- Overrides the default SGX setting for this specific run
- Enables debug logging in Gramine (`DEBUG=1`)
- Uses `gramine-direct` instead of `gramine-sgx` (`SGX=`)
- Builds the project if necessary and starts the Gramine server

Debug mode effects:

- Sets Gramine's log level to "debug" for more verbose output
- Runs the application without the SGX enclave for easier debugging

Note: Debug mode doesn't provide the same security as `gramine-sgx`. Use it only for development in a secure environment.

Logs will be displayed directly in the terminal where you run the command. If you need to run the container in detached mode and view logs separately, you can modify the command like this:

```bash
docker compose run -d --rm teleport "DEBUG=1 SGX= make start-gramine-server"
```

Then view logs with:

```bash
docker compose logs teleport
```

Remember to switch back to non-debug mode for production or when testing security features.Let me add some brief explanation about what exactly does it do.

## Code Style

We use `rustfmt` for code formatting. The configuration is in [rustfmt.toml](../rustfmt.toml).

Run before committing:

```bash
cargo fmt
```

## Common Issues

- If SGX errors occur, ensure the SGX driver is properly installed and the hardware supports SGX.
- For Twitter API issues, verify the credentials in `private.env`.

## Useful Commands

```bash
# Rebuild and restart the server
docker compose build && docker compose run --rm teleport "make start-gramine-server"

# View logs
docker compose logs teleport
```

## Resources

- [Gramine Documentation](https://gramine.readthedocs.io/)
- [Axum Documentation](https://docs.rs/axum/latest/axum/) (for web framework)
- [Rusqlite Documentation](https://docs.rs/rusqlite/latest/rusqlite/) (for SQLite database)
- [Twitter API Documentation](https://developer.twitter.com/en/docs)
- [OpenAI API Documentation](https://platform.openai.com/docs/introduction)
- [Alloy Documentation](https://github.com/alloy-rs/alloy) (for blockchain interactions)
