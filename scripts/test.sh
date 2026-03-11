#!/bin/bash
# Test script for pyx Rust implementation
set -e

echo "Running pyx tests..."

# Unit tests
cargo test --lib

echo ""
echo "✓ All tests passed!"
