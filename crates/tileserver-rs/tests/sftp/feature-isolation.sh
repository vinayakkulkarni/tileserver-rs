#!/usr/bin/env bash
# Acceptance A6: the sftp feature must build a working binary WITHOUT any
# cloud/object_store dependency leaking into the tree. Fails loudly if a
# cloud-only crate shows up while compiling --no-default-features --features sftp.
set -euo pipefail

cd "$(dirname "$0")/../../../.."

echo "→ cargo build --no-default-features --features sftp"
cargo build --no-default-features --features sftp 2>&1 | tee /tmp/sftp-feature-isolation.log

if grep -E "Compiling (object_store|aws-sdk-s3|azure_|gcp_)" /tmp/sftp-feature-isolation.log; then
    echo "FAIL: a cloud-only dependency leaked into the sftp-only build"
    exit 1
fi

echo "PASS: sftp-only build is free of cloud object-store dependencies"
