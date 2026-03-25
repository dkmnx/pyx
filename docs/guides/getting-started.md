# Getting Started

Installation and initial setup guide for pyx.

## Prerequisites

- Rust 1.75+ and Cargo
- Native secret store (libsecret-tools on Linux, Keychain on macOS, Credential Manager on Windows)
- API key for your chosen provider

## Installation

### Option 1: Prebuilt Binaries

Download the appropriate binary for your platform from [GitHub Releases](https://github.com/dkmnx/pyx/releases/latest):

**Linux (x86_64):**

```bash
curl -L -o pyx.tar.gz "https://github.com/dkmnx/pyx/releases/latest/download/pyx-$(curl -s https://api.github.com/repos/dkmnx/pyx/releases/latest | grep -oP 'tag_name": "v\K[^"]+')-x86_64-unknown-linux-gnu.tar.gz"
tar -xzf pyx.tar.gz
sudo mv pyx /usr/local/bin/
```

**macOS (Apple Silicon - M1/M2/M3):**

```bash
curl -L -o pyx.tar.gz "https://github.com/dkmnx/pyx/releases/latest/download/pyx-$(curl -s https://api.github.com/repos/dkmnx/pyx/releases/latest | grep -oP 'tag_name": "v\K[^"]+')-aarch64-apple-darwin.tar.gz"
tar -xzf pyx.tar.gz
sudo mv pyx /usr/local/bin/
```

**macOS (Intel):**

```bash
curl -L -o pyx.tar.gz "https://github.com/dkmnx/pyx/releases/latest/download/pyx-$(curl -s https://api.github.com/repos/dkmnx/pyx/releases/latest | grep -oP 'tag_name": "v\K[^"]+')-x86_64-apple-darwin.tar.gz"
tar -xzf pyx.tar.gz
sudo mv pyx /usr/local/bin/
```

**Windows (x86_64):**
Download `pyx-{version}-x86_64-pc-windows-msvc.zip` from the releases page and extract `pyx.exe` to a directory in your PATH.

### Option 2: Build from Source

```bash
git clone https://github.com/dkmnx/pyx.git
cd pyx
just install
```

## Setup

```bash
pyx setup
```

This will:

1. Create the data directory (`~/.local/share/pyx/`)
2. Generate a master encryption key
3. Store passphrase in OS keyring
4. Prompt for provider and API key

Run `pyx setup` again to add or edit providers.

## Quick Start Workflow

```mermaid
graph LR
    A[Install] --> B[pyx setup]
    B --> C[Add Provider]
    C --> D[pyx]
    D --> E[pi runs]
```

## Supported Providers

See [Providers Reference](../reference/providers.md) for complete list of built-in providers.

## Next Steps

- [Usage Guide](usage.md) - Complete command reference
- [Troubleshooting](troubleshooting.md) - Common issues
- [Security](../reference/security.md) - Encryption and best practices
- [Architecture](../reference/architecture.md) - System design
