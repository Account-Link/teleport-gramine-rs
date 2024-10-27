# Reproducible Builds
Notes on building teleport in a reproducible manner, with Nix.

## Prerequisites
* Docker Engine with the Compose plugin (`>= 2.27.1`). ([Installation
  instructions](https://docs.docker.com/engine/install/).) Note that the version of
  Compose must be 2.27.1 or higher as
  [entitlements](https://docs.docker.com/reference/compose-file/build/#entitlements) are
  needed.

*Optional*:
* [Nix](https://zero-to-nix.com/) with flakes enabled.


## Build with Docker
In order to benefit from Nix's sandbox environment in Docker, we need to [allow
entitlements](https://docs.docker.com/reference/cli/docker/buildx/build/#allow).

```shell
docker buildx create --use --name insecure-builder --buildkitd-flags '--allow-insecure-entitlement security.insecure'
```

The teleport binary can now be built with Nix, in Docker. For convenience, we show the
necessary minimal `Dockerfile` here:

```dockerfile
# syntax=docker/dockerfile:1.3-labs
FROM  nixpkgs/cachix-flakes AS nix-build
WORKDIR /usr/src/app
COPY . .
RUN --security=insecure nix build --sandbox --show-trace

FROM scratch as nix-build-output
COPY --from=nix-build /usr/src/app/result/bin/teleport .
```

Build teleport with Nix in Docker, and output the binary to the host under
`bin/docker/`.

```shell
docker buildx build \
  --allow security.insecure \
  --tag nix-teleport \
  --target nix-build-output \
  --output type=local,dest=bin/docker/ .
```


## Build and Develop with Nix
See [Zero to Nix](https://zero-to-nix.com/).

1. [Installing nix](https://zero-to-nix.com/start/install)

2. [Check installation](https://zero-to-nix.com/start/nix-run)
  ```shell
  echo "Hello Nix" | nix run "https://flakehub.com/f/NixOS/nixpkgs/*#ponysay"
  ```

3. Building teleport with `nix`
  ```shell
  nix build --out-link bin/nix/
  ```
  Binary will be under `result/bin/`

  It's possible to set a different output directory, e.g.:
  ```shell
  nix build --out-link bin/nix/
  ```

4. Development environment with `nix develop` -- start a development shell with:
  ```shell
  nix develop
  ```
  You should now have a nix-based environment. For instance, try:

  ```shell
  type cargo
  #cargo is /nix/store/mpkddnv8934wrg0jb0lh38d16pl17ss3-rust-default-1.82.0/bin/cargo
  ```


## Documentation
[Building Rust with Nix](https://github.com/NixOS/nixpkgs/blob/master/doc/languages-frameworks/rust.section.md#importing-a-cargolock-file-importing-a-cargolock-file)

[oxalica's Rust overlay](https://github.com/oxalica/rust-overlay)

## CI
https://github.com/DeterminateSystems/nix-installer-action?tab=readme-ov-file

## Troubleshooting
