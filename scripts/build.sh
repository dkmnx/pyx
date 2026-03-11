#!/bin/bash
# Build script for pyx Rust implementation
set -e

echo "Building pyx (Rust rewrite)..."

# Debug build
cargo build --release

echo ""
echo "✓ Build complete!"
echo "Binary location: target/release/pyx"
echo ""
echo "Binary size:"
ls -lh target/release/pyx
