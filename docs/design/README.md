# Design Decisions

Architecture Decision Records (ADRs) for pyx.

## Overview

This directory contains design decisions that shaped pyx's implementation.

## Key Decisions

### Age Encryption over Custom Crypto

**Decision**: Use age encryption library for all cryptographic operations

**Rationale**:

- Age is a modern, well-audited encryption format
- Provides scrypt key derivation out of the box
- Compatible with Go age implementation
- Avoids rolling custom crypto (security risk)

### OS Keyring for Passphrase Storage

**Decision**: Store passphrase in OS keyring as primary storage

**Rationale**:

- Keyring provides OS-level security
- User doesn't need to remember passphrase for each run
- Fallback to environment variable for automation
- Machine-derived encryption as last resort

### Flat File Storage over Database

**Decision**: Store encrypted data in JSON files rather than database

**Rationale**:

- Simpler deployment (no database dependency)
- Easy backup with standard tools
- Age encryption provides necessary security
- JSON is human-readable for debugging

### Provider Name Derivation Fallback

**Decision**: Derive environment variable from provider name if no mapping exists

**Rationale**:

- Reduces configuration for custom providers
- Follows convention (`provider` → `PROVIDER_API_KEY`)
- Allows custom providers with minimal setup

## Future Considerations

- SQLite for complex queries (models filtering)
- Additional encryption for machine-bound keys
- Provider API key validation during setup
