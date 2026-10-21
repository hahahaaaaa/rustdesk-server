# RustDesk Server Program

[![build](https://github.com/rustdesk/rustdesk-server/actions/workflows/build.yaml/badge.svg)](https://github.com/rustdesk/rustdesk-server/actions/workflows/build.yaml)

[**Download**](https://github.com/rustdesk/rustdesk-server/releases)

[**Manual**](https://rustdesk.com/docs/en/self-host/)

[**Configuration & environment variables**](docs/environment-variables.md)

[**FAQ**](https://github.com/rustdesk/rustdesk/wiki/FAQ)

[**How to migrate OSS to Pro**](https://rustdesk.com/docs/en/self-host/rustdesk-server-pro/installscript/#convert-from-open-source)

Self-host your own RustDesk server, it is free and open source.

> [!IMPORTANT]
> **Need more features?** [RustDesk Server Pro](https://rustdesk.com/pricing.html) might suit you better.
>
> **Want to develop your own server?** Start with [rustdesk-server-demo](https://github.com/rustdesk/rustdesk-server-demo), a simpler starting point than this repository.

## How to build manually

```bash
cargo build --release
```

Three executables will be generated in target/release.

- hbbs - RustDesk ID/Rendezvous server
- hbbr - RustDesk relay server
- rustdesk-utils - RustDesk CLI utilities

The management API and browser console are separate projects:

```bash
cargo build --release --manifest-path api/Cargo.toml
cd web-console && npm ci && npm run build
```

`rustdesk-api` listens on port `21114` by default and serves
`web-console/dist` when `API_CONSOLE_DIR` points at it. See
[Management API and web console](#management-api-and-web-console).

You can find updated binaries on the [Releases](https://github.com/rustdesk/rustdesk-server/releases) page.

## Configuration

`hbbs` and `hbbr` can be configured with command-line flags, environment
variables, or an `.env` / config file. Run `hbbs --help` or `hbbr --help` to see
the available flags.

The most common options:

| Option | Flag | Env var | Applies to | Purpose |
| --- | --- | --- | --- | --- |
| Key | `-k` | `KEY` | hbbs, hbbr | `hbbs` loads/generates one by default |
| Bind address | `-b` | `BIND` | hbbs, hbbr | Local IP address to listen on (default: all interfaces; requires 1.1.17+) |
| Port | `-p` | `PORT` | hbbs, hbbr | Listening port (hbbs `21116`, hbbr `21117`) |
| Relay servers | `-r` | `RELAY-SERVERS` | hbbs | Override when the relay uses a different address or a non-standard port |
| Force relay | — | `ALWAYS_USE_RELAY` | hbbs | `Y` disables direct connections |
| Log level | — | `RUST_LOG` | hbbs, hbbr | e.g. `debug` (default `info`) |

See **[docs/environment-variables.md](docs/environment-variables.md)** for the
full list of variables, the file/flag/env precedence rules, database and relay
bandwidth tuning, Docker image variables, and examples.

## Management API and web console

The s6 supervisor image built by the "Docker image" workflow bundles `hbbs`,
`hbbr`, `rustdesk-api`, and the browser console. It keeps the server keys and
the API database in `/data`. The multi-arch image is published to GitHub
Container Registry as `ghcr.io/hahahaaaaa/rustdesk-server:latest`
(`linux/amd64`, `linux/arm64`, `linux/arm/v7`):

```bash
# Edit RELAY and API_ADMIN_PASSWORD first.
docker compose up -d
```

If the package is private, run `docker login ghcr.io` first (use a GitHub
personal access token with `read:packages`). Packages are private by default
even when the repository is public; set the package visibility to public in
the repository's package settings if anonymous pulls are needed.

Open `http://<server>:21114/` to sign in. Set `API_ADMIN_PASSWORD` (and
optionally `API_ADMIN_USERNAME`) on the first start; if the password is omitted,
a random one is printed once in `docker logs`. Port `21114` must be reachable by
RustDesk clients because they use the API for heartbeat and configuration.

The console also provides **browser remote control**: select a device and choose
网页远控 to operate it without installing a client. The browser connects only to
the API; the API proxies the RustDesk protocol to `hbbs`/`hbbr` inside the
container. This feature needs HTTPS (WebCodecs is restricted to secure
contexts), a current Chrome or Edge, and an `hbbs` key pair (the default).
See the [web control notes](docs/environment-variables.md#browser-remote-control).

All API variables are documented in
[docs/environment-variables.md](docs/environment-variables.md#management-api--web-console-variables).

## Installation

Please follow this [doc](https://rustdesk.com/docs/en/self-host/rustdesk-server-oss/)
