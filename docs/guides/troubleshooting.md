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

The default build uses the `vendored` Cargo feature to statically link libdbus, so no system packages are needed. If building without `vendored`:

| Distro        | Command                                          |
| ------------- | ------------------------------------------------ |
| Debian/Ubuntu | `sudo apt install libdbus-1-dev pkg-config`      |
| Fedora/RHEL   | `sudo dnf install dbus-devel pkgconf-pkg-config` |
| Arch Linux    | `sudo pacman -S dbus pkgconf`                    |

### Keyring issues on Linux

pyx uses your system's Secret Service daemon for secure passphrase storage. If you see "OS keyring unavailable":

**Install a Secret Service daemon:**

| Distro        | Command                          |
| ------------- | -------------------------------- |
| Debian/Ubuntu | `sudo apt install gnome-keyring` |
| Fedora/RHEL   | `sudo dnf install gnome-keyring` |
| Arch Linux    | `sudo pacman -S gnome-keyring`   |

For headless/CI environments, `pass-secret-service` provides a lightweight alternative.

**Failing that**, set the passphrase via environment variable:

```bash
export PYX_PASSPHRASE="your-passphrase"
```

Or enable the encrypted file fallback:

```bash
export PYX_ALLOW_FILE_FALLBACK=1
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
