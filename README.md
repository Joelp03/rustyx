# 🦀 Rustyx - Reverse Proxy in Rust

Rustyx is a minimal reverse proxy written in Rust, inspired by NGINX. It routes HTTP traffic to multiple backends based on the request path, adds headers like X-Forwarded-For, and uses a simple .toml file for configuration.

## Features

- **Multi-server configuration**: Support for multiple proxy servers with different listening addresses
- **Path-based routing**: Route requests to different backend servers based on URL paths
- **Graceful Shutdown**:  Support for a "graceful shutdown" signal.
- **HTTP/HTTPS tunneling**: Full support for CONNECT method and SSL tunneling
- **Async architecture**: Built on Tokio for high concurrency and performance
- **Header preservation**: Maintains original header casing and formatting
- **Connection upgrades**: Support for WebSocket and other protocol upgrades

## Installation

### Prerequisites

- Rust 1.70+ (2024 edition)
- Cargo

### Build from source

```bash
git clone <repository-url>
cd rustyx
cargo build --release
```

## Configuration

Rustyx uses a TOML configuration file (`rustyx.toml`) to define proxy servers and routing rules.

### Example Configuration

```toml
# Primary server
[[server]]
listen = ["127.0.0.1:8000"]
name = "localhost"

[[server.location]]
path = "/"
proxy_pass = "127.0.0.1:9000"

[[server.location]]
path = "/api"
proxy_pass = "127.0.0.1:9001"

# Additional server
[[server]]
listen = ["127.0.0.1:8080", "0.0.0.0:8080"]
name = "public"

[[server.location]]
path = "/app"
proxy_pass = "127.0.0.1:3000"
```

### Configuration Options

- `listen`: Array of socket addresses to bind the proxy server
- `name`: Human-readable name for the server instance
- `location`: Array of routing rules
  - `path`: URL path prefix to match
  - `proxy_pass`: Backend server address to forward requests

## Usage

1. Create your `rustyx.toml` configuration file
2. Run the proxy server:

```bash
cargo run
```

Or with the release build:

```bash
./target/release/rustyx
```

## Architecture

### Core Components

- **Master**: Main orchestrator that manages multiple server instances
- **ProxyService**: HTTP service implementation handling request routing
- **Config**: TOML-based configuration management
- **HTTP modules**: Request/response handling and forwarding

### Request Flow

1. Client connects to a configured listening address
2. Rustyx accepts the connection and spawns a task
3. Incoming requests are matched against location paths
4. Requests are forwarded to the appropriate backend server
5. Responses are proxied back to the client

### Path Matching

The proxy uses longest-prefix matching for path routing. For example:
- Request to `/api/users` matches `/api` over `/`
- Request to `/app/dashboard` matches `/app` if configured

## Development

### Project Structure

```
src/
├── main.rs           # Application entry point
├── rustyx.rs         # Master server orchestrator
├── config/           # Configuration management
│   ├── mod.rs
│   └── config.rs
├── handlers/         # Request handlers
│   ├── mod.rs
│   └── proxy.rs      # Proxy service implementation
└── http/             # HTTP utilities
    ├── mod.rs
    ├── request.rs
    └── response.rs
```

### Dependencies

- **hyper**: HTTP implementation
- **hyper-util**: HTTP utilities and runtime
- **tokio**: Async runtime
- **serde**: Serialization framework
- **toml**: TOML parsing

### Running Tests

```bash
cargo test
```

### Development Server

For development with auto-reload:

```bash
cargo watch -x run
```

## Performance

Rustyx is designed for high performance with:
- Zero-copy request forwarding where possible
- Async I/O throughout the stack
- Efficient connection pooling
- Minimal memory allocations

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests if applicable
5. Submit a pull request

## License

[Add your license information here]

## Roadmap

- [ ] HTTPS support
- [ ] Load balancing support
- [ ] Hot configuration reload
- [ ] Health checks for backend servers
