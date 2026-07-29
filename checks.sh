#!/bin/bash

CARGO_VERSION=$(sed -nE '/^\s*version\s*=/ {s/.*"([0-9]+)\.([0-9]+)\..*/\1.\2/p; q}' Cargo.toml)
README_VERSION=$(grep -E 'camt053\s*=\s*"[^"]*"' README.md | head -1 | sed -E 's/.*"([^"]+)".*/\1/')

if [[ "$CARGO_VERSION" != "$README_VERSION" ]]; then
  echo "Cargo.toml version: $CARGO_VERSION"
  echo "README.md version:  $README_VERSION"
  echo "Version mismatch detected!"
  exit 1
fi
echo "✓ Versions match!"

cargo fmt --check > /dev/null 2>&1
if [ $? -ne 0 ]; then
    echo "✗ Format check failed. Please run 'cargo fmt' to format the code."
    exit 1
fi
echo "✓ Format check passed."

cargo clippy --all-targets --all-features -- -D warnings > /dev/null 2>&1
if [ $? -ne 0 ]; then
    echo "✗ Clippy check failed. Please fix the warnings."
    exit 1
fi
echo "✓ Clippy check passed."

cargo test --all-targets --all-features > /dev/null 2>&1
if [ $? -ne 0 ]; then
    echo "✗ Tests failed. Please fix the failing tests."
    exit 1
fi
echo "✓ Tests passed."

RUSTDOCFLAGS="-D warnings" cargo doc --all-features --no-deps > /dev/null 2>&1
if [ $? -ne 0 ]; then
    echo "✗ Documentation generation failed."
    exit 1
fi
echo "✓ Documentation generated successfully."