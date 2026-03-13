# Troubleshooting

## Installation

### "pyx: command not found"

Ensure pyx is in PATH:

```bash
which pyx
# Should show: /home/user/.cargo/bin/pyx
```

Add Cargo bin to PATH:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

### Build fails on Linux

Install libsecret for keyring support:

```bash
# Debian/Ubuntu
sudo apt install libsecret-1-dev

# Arch
sudo pacman -S libsecret
```

## Configuration

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

### "Provider not found"

List configured providers:

```bash
pyx list
```

## File Permissions

Fix data directory permissions:

```bash
chmod 700 ~/.local/share/pyx
chmod 600 ~/.local/share/pyx/*
```

## Reset Everything

```bash
pyx reset
pyx setup
```
