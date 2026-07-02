# SFTP test fixtures — THROWAWAY KEYS ONLY

Every key in this directory is a **disposable, purpose-generated test
credential** with no passphrase. They exist solely to drive the local docker
SFTP harness (`../docker-compose.yml`) and the integration tests in
`../../sftp_sources.rs`. They grant access to nothing real.

| File | Purpose |
|---|---|
| `ssh_host_ed25519_key[.pub]` / `ssh_host_rsa_key[.pub]` | Deterministic SSH host keys the container serves, so `known_hosts.good` has a stable fingerprint. |
| `id_ed25519_client[.pub]` | Test client identity. `authorized_keys/id_ed25519_client.pub` is mounted into the container. |
| `known_hosts.good` | Correct `[127.0.0.1]:2222` / `[localhost]:2222` host-key entries. |
| `known_hosts.stale` | A different host key — drives the host-key-mismatch test (I4). |

The host private keys are mode `644` (not the usual `600`) because the docker
volume is mounted read-only and sshd inside the container must read them
without being able to `chown`. This is safe **only** because these keys are
worthless test material. Never reuse this pattern for real host keys.
