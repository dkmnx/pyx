# Troubleshooting

Common issues and solutions for pyx.

## Installation Issues

### "pyx: command not found"

Ensure pyx is in PATH:

```bash
which pyx
# Should show: ~/.cargo/bin/pyx
```

Add Cargo bin to PATH:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

### Build fails on Linux

Install libsecret-tools for keyring support:

```bash
# Debian/Ubuntu
sudo apt install libsecret-tools

# Arch
sudo pacman -S libsecret
```

## Configuration Issues

### "Pyx not initialized"

Run setup:

```bash
pyx setup
```

### "No passphrase available"

Check OS keyring or set env var:

```bash
export PYX_PASSPHRASE="your-passphrase"
```

See [Security Reference](../reference/security.md) for passphrase storage options.

### "Provider not found"

List configured providers:

```bash
pyx list
```

## File Permission Issues

Fix data directory permissions:

```bash
chmod 700 ~/.local/share/pyx
chmod 600 ~/.local/share/pyx/*
```

See [Storage Reference](../reference/storage.md) for file details.

## Reset Everything

```bash
pyx reset
pyx setup
```

## Debug Information

### Verbose Output

Run with debug logging:

```bash
RUST_LOG=debug pyx list
```

### Version Info

```bash
pyx version --json
```

## Getting Help

- [Architecture](../reference/architecture.md) - System design
- [Security](../reference/security.md) - Encryption details
- [GitHub Issues](https://github.com/dkmnx/pyx/issues) - Report bugs
