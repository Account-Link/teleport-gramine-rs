# Development Guide

This guide provides essential information for developers working on the Teleport project.

## Environment Setup

### Prerequisites

- Install the latest version of Docker
- Linux distribution with SGX support (e.g., Ubuntu 20.04 LTS)
- SGX driver installed (follow manufacturer instructions)

### Environment Variables

To set up your environment, create a private `.env` file by copying the variables from [private.env.example](../private.env.example) and filling in the necessary values for each variable. This file will contain the following key variables:

- `APP_URL`: The URL where the application is hosted. This should point to the domain or IP address where users can access the Teleport service. Note that this URL may differ between local development, staging, and production environments. For local development, it might be `http://localhost:8000`, while in staging and production, it should reflect the actual deployed domain.
- `DATABASE_URL`: The connection string used to connect to the database. This typically includes the database type, username, password, host, and database name. For example, a PostgreSQL connection string might look like: `postgres://username:password@localhost:5432/mydatabase`.
- `RPC_KEY`: An API key used for blockchain RPC to interact with the blockchain.
- `NFT_MINTER_MNEMONIC`: A mnemonic phrase used to derive the private key for the NFT minting account. The first derived account from the mnemonic is used.
- `OPENAI_API_KEY`: The API key required to access OpenAI services, which is used for the safety assessment of tweets through the GPT-4o model.
- `TWITTER_API_KEY`: The API key for authenticating with the Twitter API. This is required to interact with Twitter on behalf of the user.
- `TWITTER_API_SECRET`: The API secret associated with the Twitter API credentials. This is used alongside the API key for secure authentication.

## Docker Configuration

Our project uses Docker for containerization. The main configuration files are:

- [Dockerfile](/Dockerfile): Defines how the project image is built. It includes steps for setting up the environment, installing dependencies, and building the project.
- [docker-compose.yml](/docker-compose.yml): Defines the service setup, including environment variables, volume mounts, and device mappings for SGX support.

Refer to these files directly for the most up-to-date information on our Docker setup.

## Building and Running

```bash
# Build the Docker image
docker compose build

# Start the server
docker compose run --rm teleport "make start-gramine-server"
```

## Debugging

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
