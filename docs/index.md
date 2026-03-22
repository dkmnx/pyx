# Documentation

Comprehensive documentation for the pyx CLI tool.

## Quick Navigation

| Category         | Documents                                    | Description                           |
| ---------------- | -------------------------------------------- | ------------------------------------- |
| **Guides**       | [Getting Started](guides/getting-started.md) | Installation and initial setup        |
|                  | [Usage](guides/usage.md)                     | Complete command reference            |
|                  | [Troubleshooting](guides/troubleshooting.md) | Common issues and solutions           |
| **Reference**    | [Architecture](reference/architecture.md)    | System design and component overview  |
|                  | [Security](reference/security.md)            | Encryption model and best practices   |
|                  | [Storage](reference/storage.md)              | Data files, formats, and permissions  |
|                  | [Providers](reference/providers.md)          | Supported providers and configuration |
| **Contributing** | [Contributing](contributing.md)              | Development guidelines and PR process |

## Quick Start

```bash
# Install
curl -L -o pyx.tar.gz "<release-url>" && tar -xzf pyx.tar.gz && sudo mv pyx /usr/local/bin/

# Or build from source
git clone https://github.com/dkmnx/pyx.git && cd pyx && just install

# Setup
pyx setup

# Run with all configured providers
pyx
```

## Documentation Map

```mermaid
graph LR
    A[Documentation] --> B[Guides]
    A --> C[Reference]
    A --> D[Contributing]
    
    B --> B1[Getting Started]
    B --> B2[Usage]
    B --> B3[Troubleshooting]
    
    C --> C1[Architecture]
    C --> C2[Security]
    C --> C3[Storage]
    C --> C4[Providers]
    
    D --> D1[Setup]
    D --> D2[Code Style]
    D --> D3[Adding Providers]
```

## Related

- [README](../README.md) - Project overview
- [CHANGELOG](../CHANGELOG.md) - Version history
