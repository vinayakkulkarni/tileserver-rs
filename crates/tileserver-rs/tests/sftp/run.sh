#!/usr/bin/env bash
# Bring up the SFTP test container, wait until it accepts connections, run the
# SFTP integration tests, and tear the container down. Set
# TILESERVER_KEEP_SFTP_CONTAINER=1 to leave it running for debugging.
set -euo pipefail

cd "$(dirname "$0")"
HERE="$(pwd)"
KEEP="${TILESERVER_KEEP_SFTP_CONTAINER:-0}"

cleanup() {
    if [[ "$KEEP" != "1" ]]; then
        docker compose -f docker-compose.yml down -v --remove-orphans || true
    fi
}
trap cleanup EXIT

echo "→ starting sftp container"
docker compose -f docker-compose.yml up -d

echo "→ waiting for port 2222"
for i in $(seq 1 30); do
    if (exec 3<>/dev/tcp/127.0.0.1/2222) 2>/dev/null; then
        exec 3>&- || true
        echo "  port 2222 open after ${i} attempt(s)"
        break
    fi
    sleep 1
    if [[ $i -eq 30 ]]; then
        echo "FAIL: sftp container never opened port 2222"
        docker compose -f docker-compose.yml logs
        exit 1
    fi
done

# Give sshd a moment to finish host-key setup after the port opens.
sleep 2

export TILESERVER_SFTP_TEST_HOST=127.0.0.1
export TILESERVER_SFTP_TEST_PORT=2222
export TILESERVER_SFTP_TEST_USER=test
export TILESERVER_SFTP_TEST_IDENTITY="${HERE}/fixtures/id_ed25519_client"
export TILESERVER_SFTP_TEST_KNOWN_HOSTS="${HERE}/fixtures/known_hosts.good"
export TILESERVER_SFTP_TEST_KNOWN_HOSTS_STALE="${HERE}/fixtures/known_hosts.stale"

echo "→ running cargo test --features sftp --test sftp_sources"
cd ../..
cargo test --features sftp --test sftp_sources -- --test-threads=1 --nocapture
