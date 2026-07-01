//! SFTP PMTiles source via russh + russh-sftp.
//!
//! Range reads over SFTP. Single shared session per source with
//! exponential-backoff reconnect (tracing-instrumented). Host key
//! verification is fail-closed unless `--ssh-insecure-skip-host-key-verify`
//! is set (test-only; loud warning).

use async_trait::async_trait;
use bytes::{Bytes, BytesMut};
use pmtiles::{
    AsyncBackend, AsyncPmTilesReader, BackendResponse, Compression as PmCompression, HashMapCache,
    PmtError, PmtResult, TileCoord, TileType,
};
use russh::client::{self, Handle};
use russh::keys::ssh_key::{HashAlg, PublicKey};
use russh::keys::{PrivateKeyWithHashAlg, load_secret_key};
use russh_sftp::client::SftpSession;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tokio::sync::Mutex;
use tokio::sync::RwLock;
use tokio::time::sleep;
use tracing::{info, instrument, warn};

use crate::config::SourceConfig;
use crate::error::{Result, TileServerError};
use crate::sources::{TileCompression, TileData, TileFormat, TileMetadata, TileSource};

// ─── Public URL helpers ─────────────────────────────────────────────────

const SFTP_SCHEME: &str = "sftp://";
const DEFAULT_SFTP_PORT: u16 = 22;

/// Returns `true` if the path uses the `sftp://` URL scheme.
#[must_use]
pub fn is_sftp_url(path: &str) -> bool {
    path.starts_with(SFTP_SCHEME)
}

// ─── SFTP URL parsing ────────────────────────────────────────────────────

/// A parsed `sftp://user@host[:port]/path` location plus per-source options
/// decoded from `config.options.ssh_*`.
#[derive(Debug, Clone, PartialEq, Eq)]
struct SftpLocation {
    user: String,
    host: String,
    port: u16,
    path: String,
    identity: Option<PathBuf>,
    known_hosts: Option<PathBuf>,
    strict_host_key: bool,
}

impl SftpLocation {
    /// Parse an `sftp://user@host[:port]/absolute/path` URL. Per-source
    /// `ssh_*` options (user/port/identity/known_hosts/strict) override the
    /// URL-derived defaults.
    fn parse(s: &str, options: &HashMap<String, String>) -> Result<Self> {
        let rest = s.strip_prefix(SFTP_SCHEME).ok_or_else(|| {
            TileServerError::ConfigError(format!("SFTP URL must start with 'sftp://': '{s}'"))
        })?;

        let (authority, raw_path) = rest.split_once('/').ok_or_else(|| {
            TileServerError::ConfigError(format!(
                "SFTP URL must contain an absolute remote path: '{s}'"
            ))
        })?;

        if raw_path.is_empty() {
            return Err(TileServerError::ConfigError(format!(
                "SFTP URL remote path must not be empty: '{s}'"
            )));
        }

        let (user_part, host_part) = authority.split_once('@').ok_or_else(|| {
            TileServerError::ConfigError(format!("SFTP URL must include a user: '{s}'"))
        })?;

        if user_part.is_empty() {
            return Err(TileServerError::ConfigError(format!(
                "SFTP URL user must not be empty: '{s}'"
            )));
        }
        if host_part.is_empty() {
            return Err(TileServerError::ConfigError(format!(
                "SFTP URL host must not be empty: '{s}'"
            )));
        }

        let (host, url_port) = match host_part.rsplit_once(':') {
            Some((h, p)) => {
                let port = p.parse::<u16>().map_err(|_| {
                    TileServerError::ConfigError(format!(
                        "SFTP URL has an invalid port '{p}': '{s}'"
                    ))
                })?;
                (h.to_string(), Some(port))
            }
            None => (host_part.to_string(), None),
        };

        if host.is_empty() {
            return Err(TileServerError::ConfigError(format!(
                "SFTP URL host must not be empty: '{s}'"
            )));
        }

        let user = options
            .get("ssh_user")
            .cloned()
            .unwrap_or_else(|| user_part.to_string());

        let port = options
            .get("ssh_port")
            .and_then(|p| p.parse::<u16>().ok())
            .or(url_port)
            .unwrap_or(DEFAULT_SFTP_PORT);

        let identity = options.get("ssh_identity").map(PathBuf::from);
        let known_hosts = options.get("ssh_known_hosts_path").map(PathBuf::from);
        let strict_host_key = options
            .get("ssh_strict_host_key_checking")
            .map(|v| v != "false")
            .unwrap_or(true);

        Ok(Self {
            user,
            host,
            port,
            path: format!("/{raw_path}"),
            identity,
            known_hosts,
            strict_host_key,
        })
    }
}

// ─── CLI → source bridge ─────────────────────────────────────────────────

static CLI_SSH_IDENTITY: OnceLock<Option<PathBuf>> = OnceLock::new();
static CLI_INSECURE_SKIP_HOST_KEY_VERIFY: AtomicBool = AtomicBool::new(false);

/// Store the global `--ssh-identity` value resolved from CLI/env for SFTP
/// sources to consult. Idempotent — a second call (e.g. on hot reload) is
/// ignored so the first-set value wins.
pub fn set_cli_ssh_identity(path: Option<PathBuf>) {
    let _ = CLI_SSH_IDENTITY.set(path);
}

/// Enable/disable the test-only host-key bypass from the CLI flag.
pub fn set_cli_insecure_skip_host_key_verify(value: bool) {
    CLI_INSECURE_SKIP_HOST_KEY_VERIFY.store(value, Ordering::SeqCst);
}

fn cli_ssh_identity() -> Option<PathBuf> {
    CLI_SSH_IDENTITY.get().cloned().flatten()
}

fn insecure_skip_host_key_verify() -> bool {
    CLI_INSECURE_SKIP_HOST_KEY_VERIFY.load(Ordering::SeqCst)
}

// ─── Auth resolution ─────────────────────────────────────────────────────

/// Inputs to the identity-resolution precedence chain.
struct SftpAuthOptions {
    source_identity: Option<PathBuf>,
    cli_identity: Option<PathBuf>,
    home_dir: Option<PathBuf>,
    ssh_auth_sock: Option<String>,
}

/// The resolved SSH identity used to authenticate an SFTP session.
#[derive(Debug, PartialEq, Eq)]
enum ResolvedIdentity {
    KeyFile(PathBuf),
    Agent(String),
}

/// Resolve which SSH identity to use, honouring the documented precedence:
/// per-source `ssh_identity` > `TILESERVER_SSH_IDENTITY`/`--ssh-identity` >
/// `~/.ssh/id_ed25519` > `~/.ssh/id_rsa` > `$SSH_AUTH_SOCK` agent.
fn resolve_identity(opts: &SftpAuthOptions) -> Result<ResolvedIdentity> {
    if let Some(path) = &opts.source_identity {
        require_readable(path)?;
        return Ok(ResolvedIdentity::KeyFile(path.clone()));
    }
    if let Some(path) = &opts.cli_identity {
        require_readable(path)?;
        return Ok(ResolvedIdentity::KeyFile(path.clone()));
    }
    if let Some(home) = &opts.home_dir {
        let ed25519 = home.join(".ssh").join("id_ed25519");
        if ed25519.is_file() {
            return Ok(ResolvedIdentity::KeyFile(ed25519));
        }
        let rsa = home.join(".ssh").join("id_rsa");
        if rsa.is_file() {
            return Ok(ResolvedIdentity::KeyFile(rsa));
        }
    }
    if let Some(sock) = &opts.ssh_auth_sock
        && !sock.is_empty()
    {
        return Ok(ResolvedIdentity::Agent(sock.clone()));
    }

    Err(TileServerError::SftpAuthError(format!(
        "no SSH identity found; tried: source options ssh_identity={:?}, \
         --ssh-identity/TILESERVER_SSH_IDENTITY={:?}, ~/.ssh/id_ed25519, \
         ~/.ssh/id_rsa, $SSH_AUTH_SOCK={}",
        opts.source_identity,
        opts.cli_identity,
        opts.ssh_auth_sock.as_deref().unwrap_or("<absent>"),
    )))
}

fn require_readable(path: &Path) -> Result<()> {
    std::fs::File::open(path).map(|_| ()).map_err(|e| {
        TileServerError::SftpAuthError(format!(
            "SSH identity '{}' is not readable: {e}",
            path.display()
        ))
    })
}

// ─── known_hosts parsing + host key verification ─────────────────────────

/// One parsed `known_hosts` entry. `hostnames` may contain literal names,
/// wildcard patterns (`*.example.com`), or a single hashed marker; port
/// qualification is folded into the stored name as `[host]:port`.
#[derive(Debug, Clone, PartialEq, Eq)]
struct KnownHostsEntry {
    hostnames: Vec<String>,
    hashed: Option<HashedHost>,
    key_type: String,
    public_key: Vec<u8>,
}

/// A hashed `known_hosts` host marker: HMAC-SHA1(salt, hostname) in the
/// `|1|<b64 salt>|<b64 hash>` form. We store the raw decoded salt + hash.
#[derive(Debug, Clone, PartialEq, Eq)]
struct HashedHost {
    salt: Vec<u8>,
    hash: Vec<u8>,
}

/// Outcome of comparing a received host key against `known_hosts`.
#[derive(Debug, PartialEq, Eq)]
enum HostKeyCheckOutcome {
    Match,
    Mismatch { expected: Vec<u8>, got: Vec<u8> },
    UnknownHost,
}

fn parse_known_hosts(contents: &str) -> Vec<KnownHostsEntry> {
    contents
        .lines()
        .filter_map(parse_known_hosts_line)
        .collect()
}

fn parse_known_hosts_line(line: &str) -> Option<KnownHostsEntry> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return None;
    }

    let mut fields = line.split_whitespace();
    let host_field = fields.next()?;
    // Skip an optional @cert-authority / @revoked marker.
    let (host_field, key_type) = if host_field.starts_with('@') {
        (fields.next()?, fields.next()?)
    } else {
        (host_field, fields.next()?)
    };
    let key_b64 = fields.next()?;
    let public_key = decode_base64(key_b64)?;

    if let Some(hashed) = parse_hashed_host(host_field) {
        return Some(KnownHostsEntry {
            hostnames: Vec::new(),
            hashed: Some(hashed),
            key_type: key_type.to_string(),
            public_key,
        });
    }

    let hostnames = host_field.split(',').map(str::to_string).collect();
    Some(KnownHostsEntry {
        hostnames,
        hashed: None,
        key_type: key_type.to_string(),
        public_key,
    })
}

fn parse_hashed_host(field: &str) -> Option<HashedHost> {
    let rest = field.strip_prefix("|1|")?;
    let (salt_b64, hash_b64) = rest.split_once('|')?;
    Some(HashedHost {
        salt: decode_base64(salt_b64)?,
        hash: decode_base64(hash_b64)?,
    })
}

/// Minimal standard-alphabet base64 decoder (no padding tolerance issues)
/// so the known_hosts parser stays free of an extra dependency in the
/// `--features sftp` slim build.
fn decode_base64(input: &str) -> Option<Vec<u8>> {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut lookup = [255u8; 256];
    for (i, &c) in TABLE.iter().enumerate() {
        lookup[c as usize] = i as u8;
    }

    let bytes: Vec<u8> = input.bytes().filter(|&b| b != b'=').collect();
    let mut out = Vec::with_capacity(bytes.len() * 3 / 4);
    let mut buf = 0u32;
    let mut bits = 0u32;
    for b in bytes {
        let val = lookup[b as usize];
        if val == 255 {
            return None;
        }
        buf = (buf << 6) | u32::from(val);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
        }
    }
    Some(out)
}

fn host_matches(entry: &KnownHostsEntry, host: &str, port: u16) -> bool {
    if let Some(hashed) = &entry.hashed {
        return hashed_host_matches(hashed, host)
            || hashed_host_matches(hashed, &format!("[{host}]:{port}"));
    }
    entry.hostnames.iter().any(|pattern| {
        pattern_matches(pattern, host) || pattern_matches(pattern, &format!("[{host}]:{port}"))
    })
}

fn pattern_matches(pattern: &str, host: &str) -> bool {
    if pattern == host {
        return true;
    }
    if let Some(suffix) = pattern.strip_prefix("*.") {
        return host.strip_prefix(|c: char| c != '.').is_some()
            && host
                .split_once('.')
                .map(|(_, rest)| rest == suffix)
                .unwrap_or(false);
    }
    false
}

fn hashed_host_matches(hashed: &HashedHost, host: &str) -> bool {
    hmac_sha1(&hashed.salt, host.as_bytes()) == hashed.hash
}

/// HMAC-SHA1 over `data` keyed by `salt`, as OpenSSH uses to hash
/// `known_hosts` host names. Implemented with `sha2`'s sibling isn't
/// available, so a compact self-contained SHA-1 backs it.
fn hmac_sha1(salt: &[u8], data: &[u8]) -> Vec<u8> {
    const BLOCK: usize = 64;
    let mut key = [0u8; BLOCK];
    if salt.len() > BLOCK {
        let digest = sha1(salt);
        key[..digest.len()].copy_from_slice(&digest);
    } else {
        key[..salt.len()].copy_from_slice(salt);
    }
    let mut ipad = [0x36u8; BLOCK];
    let mut opad = [0x5cu8; BLOCK];
    for i in 0..BLOCK {
        ipad[i] ^= key[i];
        opad[i] ^= key[i];
    }
    let mut inner = Vec::with_capacity(BLOCK + data.len());
    inner.extend_from_slice(&ipad);
    inner.extend_from_slice(data);
    let inner_digest = sha1(&inner);
    let mut outer = Vec::with_capacity(BLOCK + inner_digest.len());
    outer.extend_from_slice(&opad);
    outer.extend_from_slice(&inner_digest);
    sha1(&outer).to_vec()
}

/// Compact SHA-1 (RFC 3174). Used only to reproduce OpenSSH's hashed
/// `known_hosts` HMAC — SHA-1 is required by that on-disk format and is not
/// used for any security-bearing digest.
fn sha1(data: &[u8]) -> [u8; 20] {
    let mut h: [u32; 5] = [
        0x6745_2301,
        0xEFCD_AB89,
        0x98BA_DCFE,
        0x1032_5476,
        0xC3D2_E1F0,
    ];
    let ml = (data.len() as u64) * 8;
    let mut msg = data.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&ml.to_be_bytes());

    for chunk in msg.chunks_exact(64) {
        let mut w = [0u32; 80];
        for (i, word) in chunk.chunks_exact(4).enumerate() {
            w[i] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
        }
        for i in 16..80 {
            w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
        }
        let (mut a, mut b, mut c, mut d, mut e) = (h[0], h[1], h[2], h[3], h[4]);
        for (i, &wi) in w.iter().enumerate() {
            let (f, k) = match i {
                0..=19 => ((b & c) | ((!b) & d), 0x5A82_7999),
                20..=39 => (b ^ c ^ d, 0x6ED9_EBA1),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1B_BCDC),
                _ => (b ^ c ^ d, 0xCA62_C1D6),
            };
            let tmp = a
                .rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(wi);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = tmp;
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
    }

    let mut out = [0u8; 20];
    for (i, word) in h.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&word.to_be_bytes());
    }
    out
}

fn check_host_key(
    entries: &[KnownHostsEntry],
    host: &str,
    port: u16,
    received_key: &[u8],
    strict: bool,
) -> HostKeyCheckOutcome {
    if let Some(entry) = entries.iter().find(|e| host_matches(e, host, port)) {
        return if entry.public_key == received_key {
            HostKeyCheckOutcome::Match
        } else {
            HostKeyCheckOutcome::Mismatch {
                expected: entry.public_key.clone(),
                got: received_key.to_vec(),
            }
        };
    }
    if strict {
        HostKeyCheckOutcome::UnknownHost
    } else {
        HostKeyCheckOutcome::Match
    }
}

fn resolved_known_hosts_path(loc: &SftpLocation, home_dir: Option<&Path>) -> Option<PathBuf> {
    if let Some(p) = &loc.known_hosts {
        return Some(p.clone());
    }
    home_dir.map(|h| h.join(".ssh").join("known_hosts"))
}

// ─── russh Handler (host key verification hook) ──────────────────────────

struct SftpHandler {
    host: String,
    port: u16,
    entries: Vec<KnownHostsEntry>,
    strict: bool,
    insecure_skip: bool,
    verification_error: Arc<Mutex<Option<TileServerError>>>,
}

impl client::Handler for SftpHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        server_public_key: &PublicKey,
    ) -> std::result::Result<bool, Self::Error> {
        if self.insecure_skip {
            warn!(
                host = %self.host,
                "SSH host key verification bypassed (--ssh-insecure-skip-host-key-verify) — INSECURE"
            );
            return Ok(true);
        }

        let received = server_public_key.to_bytes().unwrap_or_default();
        match check_host_key(&self.entries, &self.host, self.port, &received, self.strict) {
            HostKeyCheckOutcome::Match => Ok(true),
            HostKeyCheckOutcome::Mismatch { expected, got } => {
                *self.verification_error.lock().await =
                    Some(TileServerError::SftpHostKeyMismatch {
                        host: self.host.clone(),
                        expected: fingerprint(&expected),
                        got: fingerprint(&got),
                    });
                Ok(false)
            }
            HostKeyCheckOutcome::UnknownHost => {
                *self.verification_error.lock().await =
                    Some(TileServerError::SftpHostKeyMismatch {
                        host: self.host.clone(),
                        expected: "<no known_hosts entry>".to_string(),
                        got: fingerprint(&received),
                    });
                Ok(false)
            }
        }
    }
}

fn fingerprint(key_bytes: &[u8]) -> String {
    match PublicKey::from_bytes(key_bytes) {
        Ok(key) => key.fingerprint(HashAlg::Sha256).to_string(),
        Err(_) => "<unparseable>".to_string(),
    }
}

// ─── SftpBackend (AsyncBackend impl) + reconnect backoff ─────────────────

const INITIAL_BACKOFF: Duration = Duration::from_millis(100);
const MAX_BACKOFF: Duration = Duration::from_secs(30);

/// A pmtiles [`AsyncBackend`] backed by a single reused SFTP session with
/// exponential-backoff reconnect. One backend == one live session; range
/// reads seek within the already-open remote file.
struct SftpBackend {
    loc: SftpLocation,
    identity: ResolvedIdentity,
    home_dir: Option<PathBuf>,
    session: Mutex<Option<SftpConnection>>,
    connect_attempts: AtomicU64,
    next_backoff: Mutex<Duration>,
}

struct SftpConnection {
    _handle: Handle<SftpHandler>,
    sftp: SftpSession,
}

impl SftpBackend {
    async fn connect(
        loc: SftpLocation,
        identity: ResolvedIdentity,
        home_dir: Option<PathBuf>,
    ) -> Result<Self> {
        let backend = Self {
            loc,
            identity,
            home_dir,
            session: Mutex::new(None),
            connect_attempts: AtomicU64::new(0),
            next_backoff: Mutex::new(INITIAL_BACKOFF),
        };
        // Establish the initial session eagerly so auth / host-key failures
        // surface at source-load time rather than on first tile read.
        backend.ensure_connected().await?;
        Ok(backend)
    }

    async fn ensure_connected(&self) -> Result<()> {
        let mut guard = self.session.lock().await;
        if guard.is_some() {
            return Ok(());
        }
        let conn = self.establish().await?;
        *guard = Some(conn);
        Ok(())
    }

    #[instrument(skip(self), fields(host = %self.loc.host, port = self.loc.port))]
    async fn establish(&self) -> Result<SftpConnection> {
        self.connect_attempts.fetch_add(1, Ordering::SeqCst);

        let entries = match resolved_known_hosts_path(&self.loc, self.home_dir.as_deref()) {
            Some(path) => match std::fs::read_to_string(&path) {
                Ok(contents) => parse_known_hosts(&contents),
                Err(_) => Vec::new(),
            },
            None => Vec::new(),
        };

        let verification_error = Arc::new(Mutex::new(None));
        let handler = SftpHandler {
            host: self.loc.host.clone(),
            port: self.loc.port,
            entries,
            strict: self.loc.strict_host_key,
            insecure_skip: insecure_skip_host_key_verify(),
            verification_error: Arc::clone(&verification_error),
        };

        let config = Arc::new(client::Config::default());
        let mut handle = client::connect(config, (self.loc.host.as_str(), self.loc.port), handler)
            .await
            .map_err(|e| {
                if let Some(err) = verification_error
                    .try_lock()
                    .ok()
                    .and_then(|mut g| g.take())
                {
                    err
                } else {
                    TileServerError::SftpConnectionError(format!(
                        "failed to connect to {}:{}: {e}",
                        self.loc.host, self.loc.port
                    ))
                }
            })?;

        self.authenticate(&mut handle).await?;

        let channel = handle.channel_open_session().await.map_err(|e| {
            TileServerError::SftpConnectionError(format!("failed to open SFTP channel: {e}"))
        })?;
        channel.request_subsystem(true, "sftp").await.map_err(|e| {
            TileServerError::SftpConnectionError(format!("failed to start sftp subsystem: {e}"))
        })?;
        let sftp = SftpSession::new(channel.into_stream()).await.map_err(|e| {
            TileServerError::SftpConnectionError(format!("failed to init SFTP session: {e}"))
        })?;

        info!(
            host = %self.loc.host,
            port = self.loc.port,
            "SFTP session connected"
        );

        Ok(SftpConnection {
            _handle: handle,
            sftp,
        })
    }

    async fn authenticate(&self, handle: &mut Handle<SftpHandler>) -> Result<()> {
        match &self.identity {
            ResolvedIdentity::KeyFile(path) => {
                let key = load_secret_key(path, None).map_err(|e| {
                    TileServerError::SftpAuthError(format!(
                        "failed to load SSH identity '{}': {e}",
                        path.display()
                    ))
                })?;
                let auth = handle
                    .authenticate_publickey(
                        &self.loc.user,
                        PrivateKeyWithHashAlg::new(Arc::new(key), best_hash(&self.loc)),
                    )
                    .await
                    .map_err(|e| {
                        TileServerError::SftpAuthError(format!(
                            "public-key auth failed for identity '{}': {e}",
                            path.display()
                        ))
                    })?;
                if !auth.success() {
                    return Err(TileServerError::SftpAuthError(format!(
                        "authentication rejected for user '{}' with identity '{}'",
                        self.loc.user,
                        path.display()
                    )));
                }
                Ok(())
            }
            ResolvedIdentity::Agent(sock) => self.authenticate_agent(handle, sock).await,
        }
    }

    async fn authenticate_agent(&self, handle: &mut Handle<SftpHandler>, sock: &str) -> Result<()> {
        use russh::keys::agent::client::AgentClient;

        let stream = tokio::net::UnixStream::connect(sock).await.map_err(|e| {
            TileServerError::SftpAuthError(format!("failed to connect SSH agent at '{sock}': {e}"))
        })?;
        let mut agent = AgentClient::connect(stream);
        let identities = agent.request_identities().await.map_err(|e| {
            TileServerError::SftpAuthError(format!("SSH agent listed no identities: {e}"))
        })?;
        for identity in identities {
            let russh::keys::agent::AgentIdentity::PublicKey { key, .. } = identity else {
                continue;
            };
            let auth = handle
                .authenticate_publickey_with(&self.loc.user, key, None, &mut agent)
                .await
                .map_err(|e| {
                    TileServerError::SftpAuthError(format!("SSH agent auth attempt failed: {e}"))
                })?;
            if auth.success() {
                return Ok(());
            }
        }
        Err(TileServerError::SftpAuthError(format!(
            "no SSH agent identity accepted for user '{}'",
            self.loc.user
        )))
    }

    async fn invalidate_session(&self) {
        *self.session.lock().await = None;
    }

    async fn next_backoff_delay(&self) -> Duration {
        let mut guard = self.next_backoff.lock().await;
        let delay = *guard;
        *guard = (*guard * 2).min(MAX_BACKOFF);
        delay
    }

    async fn reset_backoff(&self) {
        *self.next_backoff.lock().await = INITIAL_BACKOFF;
    }

    async fn read_once(&self, offset: usize, length: usize) -> Result<Bytes> {
        self.ensure_connected().await?;
        let guard = self.session.lock().await;
        let conn = guard
            .as_ref()
            .ok_or_else(|| TileServerError::SftpConnectionError("session missing".to_string()))?;

        let mut file = conn.sftp.open(self.loc.path.clone()).await.map_err(|e| {
            TileServerError::SftpConnectionError(format!(
                "failed to open remote '{}': {e}",
                self.loc.path
            ))
        })?;
        file.seek(std::io::SeekFrom::Start(offset as u64))
            .await
            .map_err(|e| {
                TileServerError::SftpConnectionError(format!("seek to {offset} failed: {e}"))
            })?;

        let mut buf = BytesMut::zeroed(length);
        let mut read = 0;
        while read < length {
            let n = file.read(&mut buf[read..]).await.map_err(|e| {
                TileServerError::SftpConnectionError(format!("range read failed: {e}"))
            })?;
            if n == 0 {
                break;
            }
            read += n;
        }
        buf.truncate(read);
        Ok(buf.freeze())
    }

    fn is_connection_error(err: &TileServerError) -> bool {
        matches!(err, TileServerError::SftpConnectionError(_))
    }
}

impl AsyncBackend for SftpBackend {
    #[instrument(skip(self), fields(host = %self.loc.host, path = %self.loc.path))]
    async fn read(&self, offset: usize, length: usize) -> PmtResult<BackendResponse> {
        loop {
            match self.read_once(offset, length).await {
                Ok(bytes) => {
                    self.reset_backoff().await;
                    return Ok(BackendResponse::new(bytes));
                }
                Err(e) if Self::is_connection_error(&e) => {
                    self.invalidate_session().await;
                    let delay = self.next_backoff_delay().await;
                    warn!(
                        attempt = self.connect_attempts.load(Ordering::SeqCst),
                        backoff_ms = delay.as_millis(),
                        error = %e,
                        "SFTP session lost; reconnecting with backoff"
                    );
                    sleep(delay).await;
                    continue;
                }
                Err(e) => {
                    warn!(error = %e, "SFTP range read failed (non-recoverable)");
                    return Err(PmtError::Reading(std::io::Error::other(e.to_string())));
                }
            }
        }
    }
}

/// RSA keys benefit from SHA-256 signatures; other key types ignore the
/// hash algorithm hint entirely (russh clamps it to `None`).
fn best_hash(_loc: &SftpLocation) -> Option<HashAlg> {
    Some(HashAlg::Sha256)
}

// ─── SftpPmTilesSource ───────────────────────────────────────────────────

type SftpReader = AsyncPmTilesReader<SftpBackend, HashMapCache>;

/// A PMTiles source served over SFTP.
pub struct SftpPmTilesSource {
    reader: Arc<RwLock<SftpReader>>,
    metadata: TileMetadata,
    tile_compression: TileCompression,
    native_format: TileFormat,
}

impl SftpPmTilesSource {
    /// Open a PMTiles archive over SFTP, resolving auth + host-key policy
    /// eagerly so failures surface at source-load time.
    pub async fn from_url(config: &SourceConfig) -> Result<Self> {
        let url_str = &config.path;
        info!("Opening SFTP PMTiles source: {url_str}");

        let options = config.options.clone().unwrap_or_default();
        let loc = SftpLocation::parse(url_str, &options)?;

        let home_dir = std::env::var("HOME").ok().map(PathBuf::from);
        let auth_opts = SftpAuthOptions {
            source_identity: loc.identity.clone(),
            cli_identity: cli_ssh_identity(),
            home_dir: home_dir.clone(),
            ssh_auth_sock: std::env::var("SSH_AUTH_SOCK").ok(),
        };
        let identity = resolve_identity(&auth_opts)?;

        let backend = SftpBackend::connect(loc, identity, home_dir).await?;

        let cache = HashMapCache::default();
        let reader: SftpReader = AsyncPmTilesReader::try_from_cached_source(backend, cache)
            .await
            .map_err(|e| {
                TileServerError::MetadataError(format!(
                    "failed to read PMTiles header from '{url_str}': {e}"
                ))
            })?;

        let header = reader.get_header();

        let mut format = match header.tile_type {
            TileType::Mvt => TileFormat::Pbf,
            TileType::Png => TileFormat::Png,
            TileType::Jpeg => TileFormat::Jpeg,
            TileType::Webp => TileFormat::Webp,
            TileType::Avif => TileFormat::Avif,
            TileType::Mlt => TileFormat::Mlt,
            TileType::Unknown => TileFormat::Unknown,
        };

        if format == TileFormat::Unknown
            && let Ok(coord) = TileCoord::new(header.min_zoom, 0, 0)
            && let Ok(Some(sample)) = reader.get_tile(coord).await
            && crate::sources::detect_mlt_format(&sample)
        {
            format = TileFormat::Mlt;
            info!(
                "Auto-detected MLT format for source '{}' via tile probe",
                config.id
            );
        }

        let native_format = format;
        let metadata_format = config.serve_as.unwrap_or(format);
        if config.serve_as.is_some() {
            info!(
                "Source '{}': native format {:?}, serving as {:?} (serve_as override)",
                config.id, native_format, metadata_format
            );
        }

        let tile_compression = convert_compression(header.tile_compression);

        let vector_layers = match reader.get_metadata().await {
            Ok(metadata_str) => serde_json::from_str::<serde_json::Value>(&metadata_str)
                .ok()
                .and_then(|json| json.get("vector_layers").cloned()),
            Err(_) => None,
        };

        let metadata = TileMetadata {
            id: config.id.clone(),
            name: config.name.clone().unwrap_or_else(|| config.id.clone()),
            description: config.description.clone(),
            attribution: config.attribution.clone(),
            format: metadata_format,
            minzoom: header.min_zoom,
            maxzoom: header.max_zoom,
            bounds: Some([
                header.min_longitude,
                header.min_latitude,
                header.max_longitude,
                header.max_latitude,
            ]),
            center: Some([
                header.center_longitude,
                header.center_latitude,
                header.center_zoom as f64,
            ]),
            vector_layers,
        };

        info!(
            "Loaded SFTP PMTiles source '{}': zoom {}-{}, format {:?}",
            config.id, header.min_zoom, header.max_zoom, metadata_format
        );

        Ok(Self {
            reader: Arc::new(RwLock::new(reader)),
            metadata,
            tile_compression,
            native_format,
        })
    }
}

fn convert_compression(compression: PmCompression) -> TileCompression {
    match compression {
        PmCompression::None => TileCompression::None,
        PmCompression::Gzip => TileCompression::Gzip,
        PmCompression::Brotli => TileCompression::Brotli,
        PmCompression::Zstd => TileCompression::Zstd,
        PmCompression::Unknown => TileCompression::None,
    }
}

#[async_trait]
impl TileSource for SftpPmTilesSource {
    async fn get_tile(&self, z: u8, x: u32, y: u32) -> Result<Option<TileData>> {
        let max_tile = 1u32 << z;
        if x >= max_tile || y >= max_tile {
            return Err(TileServerError::InvalidCoordinates { z, x, y });
        }

        if z < self.metadata.minzoom || z > self.metadata.maxzoom {
            return Ok(None);
        }

        let coord = match TileCoord::new(z, x, y) {
            Ok(c) => c,
            Err(_) => return Err(TileServerError::InvalidCoordinates { z, x, y }),
        };

        let reader = self.reader.read().await;

        match reader.get_tile(coord).await {
            Ok(Some(tile_data)) => Ok(Some(TileData {
                data: tile_data,
                format: self.native_format,
                compression: self.tile_compression,
            })),
            Ok(None) => Ok(None),
            Err(e) => {
                warn!("Error reading SFTP tile z={z} x={x} y={y}: {e}");
                Ok(None)
            }
        }
    }

    fn metadata(&self) -> &TileMetadata {
        &self.metadata
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect()
    }

    // ── U1/U2: is_sftp_url ────────────────────────────────────────────

    #[test]
    fn test_is_sftp_url_positive() {
        assert!(is_sftp_url("sftp://"));
        assert!(is_sftp_url("sftp://user@host/path"));
        assert!(is_sftp_url("sftp://user@host:2222/path"));
    }

    #[test]
    fn test_is_sftp_url_negative() {
        assert!(!is_sftp_url("s3://bucket/key"));
        assert!(!is_sftp_url("https://example.com/x"));
        assert!(!is_sftp_url("/local/path"));
        assert!(!is_sftp_url(""));
        assert!(!is_sftp_url("Sftp://user@host/path"));
    }

    // ── U3–U8: SftpLocation::parse ────────────────────────────────────

    #[test]
    fn test_sftp_location_parse_minimal() {
        let loc = SftpLocation::parse("sftp://user@host/file", &HashMap::new()).unwrap();
        assert_eq!(loc.user, "user");
        assert_eq!(loc.host, "host");
        assert_eq!(loc.port, 22);
        assert_eq!(loc.path, "/file");
        assert!(loc.strict_host_key);
    }

    #[test]
    fn test_sftp_location_parse_with_port() {
        let loc = SftpLocation::parse("sftp://u@h:9022/foo/bar.pmtiles", &HashMap::new()).unwrap();
        assert_eq!(loc.user, "u");
        assert_eq!(loc.host, "h");
        assert_eq!(loc.port, 9022);
        assert_eq!(loc.path, "/foo/bar.pmtiles");
    }

    #[test]
    fn test_sftp_location_parse_missing_user() {
        let err = SftpLocation::parse("sftp://host/file", &HashMap::new()).unwrap_err();
        assert!(matches!(err, TileServerError::ConfigError(_)));
    }

    #[test]
    fn test_sftp_location_parse_ipv4_host() {
        let loc = SftpLocation::parse("sftp://u@192.168.1.1/file", &HashMap::new()).unwrap();
        assert_eq!(loc.host, "192.168.1.1");
        assert_eq!(loc.port, 22);
    }

    #[test]
    fn test_sftp_location_parse_empty_path() {
        let err = SftpLocation::parse("sftp://u@h", &HashMap::new()).unwrap_err();
        assert!(matches!(err, TileServerError::ConfigError(_)));
    }

    #[test]
    fn test_sftp_location_parse_not_sftp_scheme() {
        let err = SftpLocation::parse("http://u@h/file", &HashMap::new()).unwrap_err();
        assert!(matches!(err, TileServerError::ConfigError(_)));
    }

    #[test]
    fn test_sftp_location_options_override_user_and_port() {
        let loc = SftpLocation::parse(
            "sftp://user@host/file",
            &opts(&[("ssh_user", "override"), ("ssh_port", "2200")]),
        )
        .unwrap();
        assert_eq!(loc.user, "override");
        assert_eq!(loc.port, 2200);
    }

    #[test]
    fn test_sftp_location_strict_host_key_toggle() {
        let loc = SftpLocation::parse(
            "sftp://user@host/file",
            &opts(&[("ssh_strict_host_key_checking", "false")]),
        )
        .unwrap();
        assert!(!loc.strict_host_key);
    }

    // ── U9–U16: resolve_identity precedence ───────────────────────────

    fn write_key(dir: &Path, name: &str) -> PathBuf {
        let ssh = dir.join(".ssh");
        std::fs::create_dir_all(&ssh).unwrap();
        let path = ssh.join(name);
        std::fs::write(&path, b"dummy-key").unwrap();
        path
    }

    #[test]
    fn test_resolve_identity_per_source_wins() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("source_key");
        std::fs::write(&src, b"k").unwrap();
        let cli = tmp.path().join("cli_key");
        std::fs::write(&cli, b"k").unwrap();
        let resolved = resolve_identity(&SftpAuthOptions {
            source_identity: Some(src.clone()),
            cli_identity: Some(cli),
            home_dir: None,
            ssh_auth_sock: None,
        })
        .unwrap();
        assert_eq!(resolved, ResolvedIdentity::KeyFile(src));
    }

    #[test]
    fn test_resolve_identity_cli_used_when_no_source() {
        let tmp = tempfile::tempdir().unwrap();
        let cli = tmp.path().join("cli_key");
        std::fs::write(&cli, b"k").unwrap();
        let resolved = resolve_identity(&SftpAuthOptions {
            source_identity: None,
            cli_identity: Some(cli.clone()),
            home_dir: None,
            ssh_auth_sock: None,
        })
        .unwrap();
        assert_eq!(resolved, ResolvedIdentity::KeyFile(cli));
    }

    #[test]
    fn test_resolve_identity_default_id_ed25519() {
        let tmp = tempfile::tempdir().unwrap();
        let ed = write_key(tmp.path(), "id_ed25519");
        let resolved = resolve_identity(&SftpAuthOptions {
            source_identity: None,
            cli_identity: None,
            home_dir: Some(tmp.path().to_path_buf()),
            ssh_auth_sock: None,
        })
        .unwrap();
        assert_eq!(resolved, ResolvedIdentity::KeyFile(ed));
    }

    #[test]
    fn test_resolve_identity_default_id_rsa_fallback() {
        let tmp = tempfile::tempdir().unwrap();
        let rsa = write_key(tmp.path(), "id_rsa");
        let resolved = resolve_identity(&SftpAuthOptions {
            source_identity: None,
            cli_identity: None,
            home_dir: Some(tmp.path().to_path_buf()),
            ssh_auth_sock: None,
        })
        .unwrap();
        assert_eq!(resolved, ResolvedIdentity::KeyFile(rsa));
    }

    #[test]
    fn test_resolve_identity_agent_sentinel() {
        let tmp = tempfile::tempdir().unwrap();
        let resolved = resolve_identity(&SftpAuthOptions {
            source_identity: None,
            cli_identity: None,
            home_dir: Some(tmp.path().to_path_buf()),
            ssh_auth_sock: Some("/tmp/agent.sock".to_string()),
        })
        .unwrap();
        assert_eq!(
            resolved,
            ResolvedIdentity::Agent("/tmp/agent.sock".to_string())
        );
    }

    #[test]
    fn test_resolve_identity_none_found_errors() {
        let tmp = tempfile::tempdir().unwrap();
        let err = resolve_identity(&SftpAuthOptions {
            source_identity: None,
            cli_identity: None,
            home_dir: Some(tmp.path().to_path_buf()),
            ssh_auth_sock: None,
        })
        .unwrap_err();
        match err {
            TileServerError::SftpAuthError(msg) => {
                assert!(msg.contains("id_ed25519"));
                assert!(msg.contains("SSH_AUTH_SOCK"));
            }
            other => panic!("expected SftpAuthError, got {other:?}"),
        }
    }

    #[test]
    fn test_resolve_identity_missing_source_file_errors() {
        let err = resolve_identity(&SftpAuthOptions {
            source_identity: Some(PathBuf::from("/nonexistent/key")),
            cli_identity: None,
            home_dir: None,
            ssh_auth_sock: None,
        })
        .unwrap_err();
        match err {
            TileServerError::SftpAuthError(msg) => {
                assert!(msg.contains("/nonexistent/key"));
            }
            other => panic!("expected SftpAuthError, got {other:?}"),
        }
    }

    // ── U17–U22: known_hosts parsing ──────────────────────────────────

    const ED25519_KEY: &str =
        "AAAAC3NzaC1lZDI1NTE5AAAAIB3Fp1o5Obm7VUZbxLpQ9zRr4kFsMhTVYnYbZUXKq7Zt";
    const RSA_KEY: &str = "AAAAB3NzaC1yc2EAAAADAQABAAAB";

    #[test]
    fn test_known_hosts_parse_basic() {
        let entries = parse_known_hosts(&format!("example.com ssh-ed25519 {ED25519_KEY}"));
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].hostnames, vec!["example.com".to_string()]);
        assert_eq!(entries[0].key_type, "ssh-ed25519");
    }

    #[test]
    fn test_known_hosts_parse_with_port() {
        let entries = parse_known_hosts(&format!("[example.com]:2222 ssh-ed25519 {ED25519_KEY}"));
        assert_eq!(entries.len(), 1);
        assert!(host_matches(&entries[0], "example.com", 2222));
    }

    #[test]
    fn test_known_hosts_parse_hash_comment() {
        let entries = parse_known_hosts(&format!(
            "# a comment line\n\nexample.com ssh-ed25519 {ED25519_KEY}"
        ));
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn test_known_hosts_parse_wildcard() {
        let entries = parse_known_hosts(&format!("*.example.com ssh-rsa {RSA_KEY}"));
        assert_eq!(entries.len(), 1);
        assert!(host_matches(&entries[0], "a.example.com", 22));
        assert!(!host_matches(&entries[0], "example.com", 22));
        assert!(!host_matches(&entries[0], "a.b.other.com", 22));
    }

    #[test]
    fn test_known_hosts_parse_multiple_comma_hosts() {
        let entries = parse_known_hosts(&format!("host1,host2 ssh-rsa {RSA_KEY}"));
        assert_eq!(entries.len(), 1);
        assert!(host_matches(&entries[0], "host1", 22));
        assert!(host_matches(&entries[0], "host2", 22));
        assert!(!host_matches(&entries[0], "host3", 22));
    }

    #[test]
    fn test_known_hosts_parse_hashed_hostname() {
        // |1|<salt>|<hash> where hash = HMAC-SHA1(salt, "example.com").
        let salt = b"0123456789";
        let hash = hmac_sha1(salt, b"example.com");
        let salt_b64 = encode_base64(salt);
        let hash_b64 = encode_base64(&hash);
        let line = format!("|1|{salt_b64}|{hash_b64} ssh-ed25519 {ED25519_KEY}");
        let entries = parse_known_hosts(&line);
        assert_eq!(entries.len(), 1);
        assert!(entries[0].hashed.is_some());
        assert!(host_matches(&entries[0], "example.com", 22));
        assert!(!host_matches(&entries[0], "evil.com", 22));
    }

    #[test]
    fn test_known_hosts_parse_cert_authority_marker_skipped_gracefully() {
        let entries =
            parse_known_hosts(&format!("@cert-authority *.example.com ssh-rsa {RSA_KEY}"));
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].key_type, "ssh-rsa");
    }

    // ── U23–U26: host key verification ────────────────────────────────

    fn entry_for(host: &str, key: &[u8]) -> KnownHostsEntry {
        KnownHostsEntry {
            hostnames: vec![host.to_string()],
            hashed: None,
            key_type: "ssh-ed25519".to_string(),
            public_key: key.to_vec(),
        }
    }

    #[test]
    fn test_host_key_check_match() {
        let entries = vec![entry_for("example.com", b"KEYBYTES")];
        assert_eq!(
            check_host_key(&entries, "example.com", 22, b"KEYBYTES", true),
            HostKeyCheckOutcome::Match
        );
    }

    #[test]
    fn test_host_key_check_mismatch() {
        let entries = vec![entry_for("example.com", b"EXPECTED")];
        match check_host_key(&entries, "example.com", 22, b"DIFFERENT", true) {
            HostKeyCheckOutcome::Mismatch { expected, got } => {
                assert_eq!(expected, b"EXPECTED");
                assert_eq!(got, b"DIFFERENT");
            }
            other => panic!("expected Mismatch, got {other:?}"),
        }
    }

    #[test]
    fn test_host_key_check_unknown_strict() {
        let entries = vec![entry_for("other.com", b"KEY")];
        assert_eq!(
            check_host_key(&entries, "example.com", 22, b"KEY", true),
            HostKeyCheckOutcome::UnknownHost
        );
    }

    #[test]
    fn test_host_key_check_unknown_loose() {
        let entries = vec![entry_for("other.com", b"KEY")];
        assert_eq!(
            check_host_key(&entries, "example.com", 22, b"KEY", false),
            HostKeyCheckOutcome::Match
        );
    }

    // ── U27: compression parity with cloud.rs ─────────────────────────

    #[test]
    fn test_convert_compression_all_variants() {
        assert_eq!(
            convert_compression(PmCompression::None),
            TileCompression::None
        );
        assert_eq!(
            convert_compression(PmCompression::Gzip),
            TileCompression::Gzip
        );
        assert_eq!(
            convert_compression(PmCompression::Brotli),
            TileCompression::Brotli
        );
        assert_eq!(
            convert_compression(PmCompression::Zstd),
            TileCompression::Zstd
        );
        assert_eq!(
            convert_compression(PmCompression::Unknown),
            TileCompression::None
        );
    }

    // ── U28–U29: from_url error paths ─────────────────────────────────

    fn make_config(path: &str) -> SourceConfig {
        SourceConfig {
            id: "test-sftp".to_string(),
            source_type: crate::config::SourceType::PMTiles,
            path: path.to_string(),
            name: None,
            attribution: None,
            description: None,
            resampling: None,
            layer_name: None,
            geometry_column: None,
            query: None,
            minzoom: None,
            maxzoom: None,
            serve_as: None,
            #[cfg(feature = "raster")]
            colormap: None,
            options: None,
            collection: None,
            asset_role: "visual".to_string(),
            dynamic: false,
            max_items: 100,
            stac_bbox: None,
            pixel_selection: crate::config::PixelSelectionMethod::default(),
            tile_path_template: None,
            tms: false,
            #[cfg(feature = "dem")]
            input_source: None,
            #[cfg(feature = "dem")]
            dem_encoding: crate::config::DemEncoding::Terrarium,
            #[cfg(feature = "dem")]
            dem_scale: None,
            #[cfg(feature = "dem")]
            dem_offset: None,
            #[cfg(feature = "dem")]
            dem_band: 1,
            #[cfg(feature = "dem")]
            dem_nodata_color: None,
        }
    }

    #[tokio::test]
    async fn from_url_invalid_url_returns_config_error() {
        let cfg = make_config("sftp://missing-path-host");
        let err = SftpPmTilesSource::from_url(&cfg)
            .await
            .err()
            .expect("malformed SFTP URL must fail before any network call");
        assert!(matches!(err, TileServerError::ConfigError(_)));
    }

    #[tokio::test]
    async fn from_url_non_sftp_scheme_returns_config_error() {
        let cfg = make_config("http://example.com/tiles.pmtiles");
        let err = SftpPmTilesSource::from_url(&cfg)
            .await
            .err()
            .expect("non-sftp URL must fail parse");
        assert!(matches!(err, TileServerError::ConfigError(_)));
    }

    // ── backoff helper ─────────────────────────────────────────────────

    #[tokio::test]
    async fn backoff_grows_then_resets() {
        let backend = SftpBackend {
            loc: SftpLocation::parse("sftp://u@h/f", &HashMap::new()).unwrap(),
            identity: ResolvedIdentity::Agent("x".to_string()),
            home_dir: None,
            session: Mutex::new(None),
            connect_attempts: AtomicU64::new(0),
            next_backoff: Mutex::new(INITIAL_BACKOFF),
        };
        let first = backend.next_backoff_delay().await;
        let second = backend.next_backoff_delay().await;
        assert_eq!(first, INITIAL_BACKOFF);
        assert_eq!(second, INITIAL_BACKOFF * 2);
        backend.reset_backoff().await;
        let third = backend.next_backoff_delay().await;
        assert_eq!(third, INITIAL_BACKOFF);
    }

    // Base64 encoder used only by the hashed-known_hosts test above.
    fn encode_base64(input: &[u8]) -> String {
        const TABLE: &[u8; 64] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut out = String::new();
        for chunk in input.chunks(3) {
            let b = [
                chunk[0],
                *chunk.get(1).unwrap_or(&0),
                *chunk.get(2).unwrap_or(&0),
            ];
            let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
            out.push(TABLE[((n >> 18) & 63) as usize] as char);
            out.push(TABLE[((n >> 12) & 63) as usize] as char);
            if chunk.len() > 1 {
                out.push(TABLE[((n >> 6) & 63) as usize] as char);
            } else {
                out.push('=');
            }
            if chunk.len() > 2 {
                out.push(TABLE[(n & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
        out
    }

    #[test]
    fn test_sha1_known_vector() {
        // SHA-1("abc") = a9993e364706816aba3e25717850c26c9cd0d89d
        let digest = sha1(b"abc");
        let hex: String = digest.iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(hex, "a9993e364706816aba3e25717850c26c9cd0d89d");
    }
}
