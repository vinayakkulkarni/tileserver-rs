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
static GLOBAL_KNOWN_HOSTS: OnceLock<Option<PathBuf>> = OnceLock::new();

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

/// Store the global `[sftp].known_hosts_path` default from the config file.
/// Per-source `options.ssh_known_hosts_path` still overrides it.
pub fn set_global_known_hosts_path(path: Option<PathBuf>) {
    let _ = GLOBAL_KNOWN_HOSTS.set(path);
}

fn global_known_hosts_path() -> Option<PathBuf> {
    GLOBAL_KNOWN_HOSTS.get().cloned().flatten()
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
    if let Some(p) = global_known_hosts_path() {
        return Some(p);
    }
    home_dir.map(|h| h.join(".ssh").join("known_hosts"))
}

/// Read and parse a `known_hosts` file, yielding an empty set when the path
/// is absent or unreadable (an unreadable file is treated as "no known
/// hosts", matching OpenSSH's tolerance of a missing file).
fn load_known_hosts_entries(path: Option<PathBuf>) -> Vec<KnownHostsEntry> {
    let Some(path) = path else {
        return Vec::new();
    };
    let Ok(contents) = std::fs::read_to_string(&path) else {
        return Vec::new();
    };
    parse_known_hosts(&contents)
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

/// Drain the host-key verification error captured by the russh handler, if
/// one was recorded during the connect handshake.
fn take_verification_error(slot: &Arc<Mutex<Option<TileServerError>>>) -> Option<TileServerError> {
    slot.try_lock().ok().and_then(|mut g| g.take())
}

/// Open the SFTP subsystem on an authenticated SSH session and hand back the
/// live [`SftpSession`].
async fn open_sftp_session(handle: &mut Handle<SftpHandler>) -> Result<SftpSession> {
    let channel = handle.channel_open_session().await.map_err(|e| {
        TileServerError::SftpConnectionError(format!("failed to open SFTP channel: {e}"))
    })?;
    channel.request_subsystem(true, "sftp").await.map_err(|e| {
        TileServerError::SftpConnectionError(format!("failed to start sftp subsystem: {e}"))
    })?;
    SftpSession::new(channel.into_stream()).await.map_err(|e| {
        TileServerError::SftpConnectionError(format!("failed to init SFTP session: {e}"))
    })
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

        let entries = load_known_hosts_entries(resolved_known_hosts_path(
            &self.loc,
            self.home_dir.as_deref(),
        ));

        let verification_error = Arc::new(Mutex::new(None));
        let handler = SftpHandler {
            host: self.loc.host.clone(),
            port: self.loc.port,
            entries,
            strict: self.loc.strict_host_key,
            insecure_skip: insecure_skip_host_key_verify(),
            verification_error: Arc::clone(&verification_error),
        };

        let mut handle = self
            .connect_and_authenticate(handler, &verification_error)
            .await?;
        let sftp = open_sftp_session(&mut handle).await?;

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

    async fn connect_and_authenticate(
        &self,
        handler: SftpHandler,
        verification_error: &Arc<Mutex<Option<TileServerError>>>,
    ) -> Result<Handle<SftpHandler>> {
        let config = Arc::new(client::Config::default());
        let mut handle = client::connect(config, (self.loc.host.as_str(), self.loc.port), handler)
            .await
            .map_err(|e| {
                take_verification_error(verification_error).unwrap_or_else(|| {
                    TileServerError::SftpConnectionError(format!(
                        "failed to connect to {}:{}: {e}",
                        self.loc.host, self.loc.port
                    ))
                })
            })?;
        self.authenticate(&mut handle).await?;
        Ok(handle)
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
        let mut file = self.open_remote_file().await?;
        seek_to(&mut file, offset).await?;
        read_range_into_buf(&mut file, length).await
    }

    async fn open_remote_file(&self) -> Result<russh_sftp::client::fs::File> {
        self.ensure_connected().await?;
        let guard = self.session.lock().await;
        let conn = guard
            .as_ref()
            .ok_or_else(|| TileServerError::SftpConnectionError("session missing".to_string()))?;
        conn.sftp.open(self.loc.path.clone()).await.map_err(|e| {
            TileServerError::SftpConnectionError(format!(
                "failed to open remote '{}': {e}",
                self.loc.path
            ))
        })
    }

    fn is_connection_error(err: &TileServerError) -> bool {
        matches!(err, TileServerError::SftpConnectionError(_))
    }
}

/// Seek `file` to `offset` from the start, mapping any IO failure to a
/// connection error so the reconnect path can retry.
async fn seek_to<S>(file: &mut S, offset: usize) -> Result<()>
where
    S: AsyncSeekExt + Unpin,
{
    file.seek(std::io::SeekFrom::Start(offset as u64))
        .await
        .map(|_| ())
        .map_err(|e| TileServerError::SftpConnectionError(format!("seek to {offset} failed: {e}")))
}

/// Read exactly `length` bytes (or until EOF) from `file` into a fresh
/// buffer. A short read at EOF truncates the returned bytes.
async fn read_range_into_buf<R>(file: &mut R, length: usize) -> Result<Bytes>
where
    R: AsyncReadExt + Unpin,
{
    let mut buf = BytesMut::zeroed(length);
    let mut read = 0;
    while read < length {
        let n = file
            .read(&mut buf[read..])
            .await
            .map_err(|e| TileServerError::SftpConnectionError(format!("range read failed: {e}")))?;
        if n == 0 {
            break;
        }
        read += n;
    }
    buf.truncate(read);
    Ok(buf.freeze())
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

/// Scalar view of the PMTiles header fields consumed when assembling
/// [`TileMetadata`]. Extracted so metadata assembly is unit-testable without
/// constructing a private pmtiles `Header`.
struct PmHeaderMeta {
    minzoom: u8,
    maxzoom: u8,
    bounds: [f64; 4],
    center: [f64; 3],
}

/// Map a pmtiles [`TileType`] to the server's [`TileFormat`].
fn tile_type_to_format(tile_type: TileType) -> TileFormat {
    match tile_type {
        TileType::Mvt => TileFormat::Pbf,
        TileType::Png => TileFormat::Png,
        TileType::Jpeg => TileFormat::Jpeg,
        TileType::Webp => TileFormat::Webp,
        TileType::Avif => TileFormat::Avif,
        TileType::Mlt => TileFormat::Mlt,
        TileType::Unknown => TileFormat::Unknown,
    }
}

/// Resolve the format advertised in metadata: the per-source `serve_as`
/// override when present, otherwise the natively-detected format.
fn resolve_metadata_format(serve_as: Option<TileFormat>, native: TileFormat) -> TileFormat {
    serve_as.unwrap_or(native)
}

/// Pull the `vector_layers` array out of a PMTiles metadata JSON string,
/// tolerating malformed JSON and a missing key by returning `None`.
fn extract_vector_layers(metadata_str: &str) -> Option<serde_json::Value> {
    serde_json::from_str::<serde_json::Value>(metadata_str)
        .ok()
        .and_then(|json| json.get("vector_layers").cloned())
}

fn build_tile_metadata(
    config: &SourceConfig,
    header: &PmHeaderMeta,
    metadata_format: TileFormat,
    vector_layers: Option<serde_json::Value>,
) -> TileMetadata {
    TileMetadata {
        id: config.id.clone(),
        name: config.name.clone().unwrap_or_else(|| config.id.clone()),
        description: config.description.clone(),
        attribution: config.attribution.clone(),
        format: metadata_format,
        minzoom: header.minzoom,
        maxzoom: header.maxzoom,
        bounds: Some(header.bounds),
        center: Some(header.center),
        vector_layers,
    }
}

/// Probe the lowest-zoom tile to decide whether an `Unknown`-typed archive is
/// actually MLT. Any read failure resolves to `false`.
async fn probe_is_mlt(reader: &SftpReader, min_zoom: u8) -> bool {
    let Ok(coord) = TileCoord::new(min_zoom, 0, 0) else {
        return false;
    };
    let Ok(Some(sample)) = reader.get_tile(coord).await else {
        return false;
    };
    crate::sources::detect_mlt_format(&sample)
}

/// Resolve the native tile format, upgrading an `Unknown` header type to MLT
/// when a tile probe detects the MLT signature.
async fn resolve_format(reader: &SftpReader, min_zoom: u8, initial: TileFormat) -> TileFormat {
    if initial != TileFormat::Unknown {
        return initial;
    }
    if probe_is_mlt(reader, min_zoom).await {
        TileFormat::Mlt
    } else {
        TileFormat::Unknown
    }
}

/// Fetch and parse the source's `vector_layers` metadata, folding any read or
/// parse failure to `None`.
async fn fetch_vector_layers(reader: &SftpReader) -> Option<serde_json::Value> {
    let Ok(metadata_str) = reader.get_metadata().await else {
        return None;
    };
    extract_vector_layers(&metadata_str)
}

/// Parse the URL, resolve the SSH identity, and open the live SFTP session,
/// surfacing auth / host-key failures eagerly at source-load time.
async fn build_backend(config: &SourceConfig) -> Result<SftpBackend> {
    let options = config.options.clone().unwrap_or_default();
    let loc = SftpLocation::parse(&config.path, &options)?;

    let home_dir = std::env::var("HOME").ok().map(PathBuf::from);
    let auth_opts = SftpAuthOptions {
        source_identity: loc.identity.clone(),
        cli_identity: cli_ssh_identity(),
        home_dir: home_dir.clone(),
        ssh_auth_sock: std::env::var("SSH_AUTH_SOCK").ok(),
    };
    let identity = resolve_identity(&auth_opts)?;

    SftpBackend::connect(loc, identity, home_dir).await
}

async fn open_reader(backend: SftpBackend, url_str: &str) -> Result<SftpReader> {
    let cache = HashMapCache::default();
    AsyncPmTilesReader::try_from_cached_source(backend, cache)
        .await
        .map_err(|e| {
            TileServerError::MetadataError(format!(
                "failed to read PMTiles header from '{url_str}': {e}"
            ))
        })
}

impl SftpPmTilesSource {
    /// Open a PMTiles archive over SFTP, resolving auth + host-key policy
    /// eagerly so failures surface at source-load time.
    pub async fn from_url(config: &SourceConfig) -> Result<Self> {
        let url_str = &config.path;
        info!("Opening SFTP PMTiles source: {url_str}");

        let backend = build_backend(config).await?;
        let reader = open_reader(backend, url_str).await?;

        let header = reader.get_header();
        let header_meta = PmHeaderMeta {
            minzoom: header.min_zoom,
            maxzoom: header.max_zoom,
            bounds: [
                header.min_longitude,
                header.min_latitude,
                header.max_longitude,
                header.max_latitude,
            ],
            center: [
                header.center_longitude,
                header.center_latitude,
                header.center_zoom as f64,
            ],
        };
        let tile_compression = convert_compression(header.tile_compression);
        let initial_format = tile_type_to_format(header.tile_type);
        let min_zoom = header.min_zoom;

        let native_format = resolve_format(&reader, min_zoom, initial_format).await;
        let metadata_format = resolve_metadata_format(config.serve_as, native_format);
        log_format_resolution(config, native_format, metadata_format);

        let vector_layers = fetch_vector_layers(&reader).await;
        let metadata = build_tile_metadata(config, &header_meta, metadata_format, vector_layers);

        info!(
            "Loaded SFTP PMTiles source '{}': zoom {}-{}, format {:?}",
            config.id, header_meta.minzoom, header_meta.maxzoom, metadata_format
        );

        Ok(Self {
            reader: Arc::new(RwLock::new(reader)),
            metadata,
            tile_compression,
            native_format,
        })
    }
}

/// Emit the `serve_as` override log line when the source advertises a
/// different format than it stores natively.
fn log_format_resolution(
    config: &SourceConfig,
    native_format: TileFormat,
    metadata_format: TileFormat,
) {
    if config.serve_as.is_some() {
        info!(
            "Source '{}': native format {:?}, serving as {:?} (serve_as override)",
            config.id, native_format, metadata_format
        );
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

/// Validate a tile request against the source's zoom range. Returns
/// `Ok(Some(coord))` for an in-range request, `Ok(None)` when the zoom is
/// outside `[minzoom, maxzoom]`, and `Err(InvalidCoordinates)` when `x`/`y`
/// exceed the `2^z` grid or `TileCoord` rejects the triple.
fn validate_tile_coord(
    z: u8,
    x: u32,
    y: u32,
    minzoom: u8,
    maxzoom: u8,
) -> Result<Option<TileCoord>> {
    let max_tile = 1u32 << z;
    if x >= max_tile || y >= max_tile {
        return Err(TileServerError::InvalidCoordinates { z, x, y });
    }
    if z < minzoom || z > maxzoom {
        return Ok(None);
    }
    match TileCoord::new(z, x, y) {
        Ok(coord) => Ok(Some(coord)),
        Err(_) => Err(TileServerError::InvalidCoordinates { z, x, y }),
    }
}

/// Map a pmtiles read result into an optional [`TileData`]. A read error is
/// logged and folded to `None` so a transient tile failure is not fatal.
fn map_tile_read(
    result: PmtResult<Option<Bytes>>,
    format: TileFormat,
    compression: TileCompression,
    z: u8,
    x: u32,
    y: u32,
) -> Option<TileData> {
    match result {
        Ok(Some(data)) => Some(TileData {
            data,
            format,
            compression,
        }),
        Ok(None) => None,
        Err(e) => {
            warn!("Error reading SFTP tile z={z} x={x} y={y}: {e}");
            None
        }
    }
}

#[async_trait]
impl TileSource for SftpPmTilesSource {
    async fn get_tile(&self, z: u8, x: u32, y: u32) -> Result<Option<TileData>> {
        let Some(coord) =
            validate_tile_coord(z, x, y, self.metadata.minzoom, self.metadata.maxzoom)?
        else {
            return Ok(None);
        };

        let reader = self.reader.read().await;
        let result = reader.get_tile(coord).await;
        Ok(map_tile_read(
            result,
            self.native_format,
            self.tile_compression,
            z,
            x,
            y,
        ))
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
    fn test_sftp_location_parse_empty_remote_path() {
        let err = SftpLocation::parse("sftp://u@h/", &HashMap::new()).unwrap_err();
        assert!(matches!(err, TileServerError::ConfigError(_)));
    }

    #[test]
    fn test_sftp_location_parse_empty_user() {
        let err = SftpLocation::parse("sftp://@h/file", &HashMap::new()).unwrap_err();
        assert!(matches!(err, TileServerError::ConfigError(_)));
    }

    #[test]
    fn test_sftp_location_parse_empty_host() {
        let err = SftpLocation::parse("sftp://u@/file", &HashMap::new()).unwrap_err();
        assert!(matches!(err, TileServerError::ConfigError(_)));
    }

    #[test]
    fn test_sftp_location_parse_invalid_port() {
        let err = SftpLocation::parse("sftp://u@h:notaport/file", &HashMap::new()).unwrap_err();
        assert!(matches!(err, TileServerError::ConfigError(_)));
    }

    #[test]
    fn test_sftp_location_parse_empty_host_before_port() {
        let err = SftpLocation::parse("sftp://u@:2222/file", &HashMap::new()).unwrap_err();
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
    fn test_resolve_identity_empty_agent_sock_errors() {
        let tmp = tempfile::tempdir().unwrap();
        let err = resolve_identity(&SftpAuthOptions {
            source_identity: None,
            cli_identity: None,
            home_dir: Some(tmp.path().to_path_buf()),
            ssh_auth_sock: Some(String::new()),
        })
        .unwrap_err();
        assert!(matches!(err, TileServerError::SftpAuthError(_)));
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

    #[test]
    fn test_resolved_known_hosts_prefers_per_source_over_home() {
        let loc = SftpLocation::parse(
            "sftp://u@h/f",
            &opts(&[("ssh_known_hosts_path", "/etc/per_source_known_hosts")]),
        )
        .unwrap();
        let resolved = resolved_known_hosts_path(&loc, Some(Path::new("/home/tester")));
        assert_eq!(
            resolved,
            Some(PathBuf::from("/etc/per_source_known_hosts")),
            "per-source ssh_known_hosts_path must win over the home fallback"
        );
    }

    #[test]
    fn test_resolved_known_hosts_falls_back_to_home() {
        let loc = SftpLocation::parse("sftp://u@h/f", &HashMap::new()).unwrap();
        let resolved = resolved_known_hosts_path(&loc, Some(Path::new("/home/tester")));
        assert_eq!(
            resolved,
            Some(PathBuf::from("/home/tester/.ssh/known_hosts"))
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

    #[test]
    fn test_sha1_empty_input() {
        // SHA-1("") = da39a3ee5e6b4b0d3255bfef95601890afd80709
        let digest = sha1(b"");
        let hex: String = digest.iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(hex, "da39a3ee5e6b4b0d3255bfef95601890afd80709");
    }

    #[test]
    fn test_decode_base64_rejects_invalid_char() {
        assert_eq!(decode_base64("****"), None);
    }

    #[test]
    fn test_hmac_sha1_long_salt_is_prehashed() {
        // A salt longer than the 64-byte block exercises the pre-hash path.
        let long_salt = vec![0xABu8; 100];
        let mac = hmac_sha1(&long_salt, b"example.com");
        assert_eq!(mac.len(), 20);
        // Deterministic: same inputs yield the same MAC.
        assert_eq!(mac, hmac_sha1(&long_salt, b"example.com"));
        assert_ne!(mac, hmac_sha1(&long_salt, b"other.com"));
    }

    // ── U30–U36: tile_type_to_format (all 7 TileType variants) ────────

    #[test]
    fn test_tile_type_to_format_mvt() {
        assert_eq!(tile_type_to_format(TileType::Mvt), TileFormat::Pbf);
    }

    #[test]
    fn test_tile_type_to_format_png() {
        assert_eq!(tile_type_to_format(TileType::Png), TileFormat::Png);
    }

    #[test]
    fn test_tile_type_to_format_jpeg() {
        assert_eq!(tile_type_to_format(TileType::Jpeg), TileFormat::Jpeg);
    }

    #[test]
    fn test_tile_type_to_format_webp() {
        assert_eq!(tile_type_to_format(TileType::Webp), TileFormat::Webp);
    }

    #[test]
    fn test_tile_type_to_format_avif() {
        assert_eq!(tile_type_to_format(TileType::Avif), TileFormat::Avif);
    }

    #[test]
    fn test_tile_type_to_format_mlt() {
        assert_eq!(tile_type_to_format(TileType::Mlt), TileFormat::Mlt);
    }

    #[test]
    fn test_tile_type_to_format_unknown() {
        assert_eq!(tile_type_to_format(TileType::Unknown), TileFormat::Unknown);
    }

    // ── U37–U38: resolve_metadata_format ──────────────────────────────

    #[test]
    fn test_resolve_metadata_format_uses_serve_as_override() {
        assert_eq!(
            resolve_metadata_format(Some(TileFormat::Png), TileFormat::Pbf),
            TileFormat::Png
        );
    }

    #[test]
    fn test_resolve_metadata_format_defaults_to_native() {
        assert_eq!(
            resolve_metadata_format(None, TileFormat::Mlt),
            TileFormat::Mlt
        );
    }

    // ── U39–U42: extract_vector_layers ────────────────────────────────

    #[test]
    fn test_extract_vector_layers_present() {
        let json = r#"{"vector_layers":[{"id":"roads"}]}"#;
        let layers = extract_vector_layers(json).expect("vector_layers must be extracted");
        assert!(layers.is_array());
        assert_eq!(layers[0]["id"], "roads");
    }

    #[test]
    fn test_extract_vector_layers_absent_key() {
        let json = r#"{"name":"basemap"}"#;
        assert_eq!(extract_vector_layers(json), None);
    }

    #[test]
    fn test_extract_vector_layers_invalid_json() {
        assert_eq!(extract_vector_layers("not json {"), None);
    }

    #[test]
    fn test_extract_vector_layers_empty_object() {
        assert_eq!(extract_vector_layers("{}"), None);
    }

    // ── U43–U45: build_tile_metadata ──────────────────────────────────

    #[test]
    fn test_build_tile_metadata_uses_config_name() {
        let mut cfg = make_config("sftp://u@h/f.pmtiles");
        cfg.name = Some("Named Source".to_string());
        let hdr = PmHeaderMeta {
            minzoom: 2,
            maxzoom: 14,
            bounds: [-1.0, -2.0, 3.0, 4.0],
            center: [0.5, 0.6, 7.0],
        };
        let meta = build_tile_metadata(&cfg, &hdr, TileFormat::Pbf, None);
        assert_eq!(meta.name, "Named Source");
        assert_eq!(meta.id, "test-sftp");
        assert_eq!(meta.minzoom, 2);
        assert_eq!(meta.maxzoom, 14);
        assert_eq!(meta.format, TileFormat::Pbf);
        assert_eq!(meta.bounds, Some([-1.0, -2.0, 3.0, 4.0]));
        assert_eq!(meta.center, Some([0.5, 0.6, 7.0]));
    }

    #[test]
    fn test_build_tile_metadata_falls_back_to_id_for_name() {
        let cfg = make_config("sftp://u@h/f.pmtiles");
        let hdr = PmHeaderMeta {
            minzoom: 0,
            maxzoom: 22,
            bounds: [0.0, 0.0, 0.0, 0.0],
            center: [0.0, 0.0, 0.0],
        };
        let meta = build_tile_metadata(&cfg, &hdr, TileFormat::Webp, None);
        assert_eq!(meta.name, "test-sftp");
    }

    #[test]
    fn test_build_tile_metadata_carries_vector_layers() {
        let cfg = make_config("sftp://u@h/f.pmtiles");
        let hdr = PmHeaderMeta {
            minzoom: 0,
            maxzoom: 10,
            bounds: [0.0, 0.0, 0.0, 0.0],
            center: [0.0, 0.0, 0.0],
        };
        let layers = serde_json::json!([{"id": "water"}]);
        let meta = build_tile_metadata(&cfg, &hdr, TileFormat::Pbf, Some(layers.clone()));
        assert_eq!(meta.vector_layers, Some(layers));
    }

    // ── U46–U51: validate_tile_coord (every branch) ───────────────────

    #[test]
    fn test_validate_tile_coord_valid() {
        let coord = validate_tile_coord(2, 1, 1, 0, 14)
            .expect("valid coords must not error")
            .expect("in-range coords must yield a coordinate");
        assert_eq!(coord, TileCoord::new(2, 1, 1).unwrap());
    }

    #[test]
    fn test_validate_tile_coord_x_out_of_bounds() {
        let err = validate_tile_coord(2, 4, 0, 0, 14).unwrap_err();
        assert!(matches!(
            err,
            TileServerError::InvalidCoordinates { z: 2, x: 4, y: 0 }
        ));
    }

    #[test]
    fn test_validate_tile_coord_y_out_of_bounds() {
        let err = validate_tile_coord(2, 0, 4, 0, 14).unwrap_err();
        assert!(matches!(
            err,
            TileServerError::InvalidCoordinates { z: 2, x: 0, y: 4 }
        ));
    }

    #[test]
    fn test_validate_tile_coord_below_minzoom() {
        assert_eq!(validate_tile_coord(1, 0, 0, 5, 14).unwrap(), None);
    }

    #[test]
    fn test_validate_tile_coord_above_maxzoom() {
        assert_eq!(validate_tile_coord(15, 0, 0, 0, 14).unwrap(), None);
    }

    #[test]
    fn test_validate_tile_coord_boundaries_inclusive() {
        assert!(validate_tile_coord(5, 0, 0, 5, 14).unwrap().is_some());
        assert!(validate_tile_coord(14, 0, 0, 5, 14).unwrap().is_some());
    }

    // ── U52–U54: map_tile_read (Some / None / Err arms) ───────────────

    #[test]
    fn test_map_tile_read_some_wraps_tile_data() {
        let bytes = Bytes::from_static(b"tile-bytes");
        let out = map_tile_read(
            Ok(Some(bytes.clone())),
            TileFormat::Pbf,
            TileCompression::Gzip,
            3,
            1,
            2,
        );
        let tile = out.expect("Some(bytes) must yield TileData");
        assert_eq!(tile.data, bytes);
        assert_eq!(tile.format, TileFormat::Pbf);
        assert_eq!(tile.compression, TileCompression::Gzip);
    }

    #[test]
    fn test_map_tile_read_none_is_none() {
        let out = map_tile_read(Ok(None), TileFormat::Pbf, TileCompression::None, 0, 0, 0);
        assert!(out.is_none());
    }

    #[test]
    fn test_map_tile_read_err_is_none() {
        let out = map_tile_read(
            Err(PmtError::InvalidMagicNumber),
            TileFormat::Pbf,
            TileCompression::None,
            0,
            0,
            0,
        );
        assert!(out.is_none());
    }

    // ── U55–U57: load_known_hosts_entries ─────────────────────────────

    #[test]
    fn test_load_known_hosts_entries_none_path() {
        assert!(load_known_hosts_entries(None).is_empty());
    }

    #[test]
    fn test_load_known_hosts_entries_unreadable_path() {
        let entries = load_known_hosts_entries(Some(PathBuf::from("/nonexistent/known_hosts")));
        assert!(entries.is_empty());
    }

    #[test]
    fn test_load_known_hosts_entries_valid_file() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("known_hosts");
        std::fs::write(&path, format!("example.com ssh-ed25519 {ED25519_KEY}")).unwrap();
        let entries = load_known_hosts_entries(Some(path));
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].hostnames, vec!["example.com".to_string()]);
    }

    // ── U58–U59: take_verification_error ──────────────────────────────

    #[tokio::test]
    async fn test_take_verification_error_present() {
        let slot: Arc<Mutex<Option<TileServerError>>> = Arc::new(Mutex::new(Some(
            TileServerError::SftpConnectionError("boom".to_string()),
        )));
        let taken = take_verification_error(&slot);
        assert!(matches!(
            taken,
            Some(TileServerError::SftpConnectionError(_))
        ));
        assert!(slot.lock().await.is_none());
    }

    #[test]
    fn test_take_verification_error_absent() {
        let slot: Arc<Mutex<Option<TileServerError>>> = Arc::new(Mutex::new(None));
        assert!(take_verification_error(&slot).is_none());
    }

    // ── U60–U62: seek_to ──────────────────────────────────────────────

    #[tokio::test]
    async fn test_seek_to_positions_cursor() {
        let mut cursor = std::io::Cursor::new(b"0123456789".to_vec());
        seek_to(&mut cursor, 4).await.unwrap();
        let mut rest = Vec::new();
        cursor.read_to_end(&mut rest).await.unwrap();
        assert_eq!(rest, b"456789");
    }

    #[tokio::test]
    async fn test_seek_to_zero_is_start() {
        let mut cursor = std::io::Cursor::new(b"abc".to_vec());
        seek_to(&mut cursor, 0).await.unwrap();
        let mut rest = Vec::new();
        cursor.read_to_end(&mut rest).await.unwrap();
        assert_eq!(rest, b"abc");
    }

    // ── U63–U66: read_range_into_buf (full read / EOF short read / empty) ──

    #[tokio::test]
    async fn test_read_range_into_buf_full_length() {
        let mut cursor = std::io::Cursor::new(b"HELLOWORLD".to_vec());
        let bytes = read_range_into_buf(&mut cursor, 5).await.unwrap();
        assert_eq!(bytes.as_ref(), b"HELLO");
    }

    #[tokio::test]
    async fn test_read_range_into_buf_short_read_truncates_at_eof() {
        let mut cursor = std::io::Cursor::new(b"HI".to_vec());
        let bytes = read_range_into_buf(&mut cursor, 8).await.unwrap();
        assert_eq!(bytes.as_ref(), b"HI");
    }

    #[tokio::test]
    async fn test_read_range_into_buf_zero_length() {
        let mut cursor = std::io::Cursor::new(b"data".to_vec());
        let bytes = read_range_into_buf(&mut cursor, 0).await.unwrap();
        assert!(bytes.is_empty());
    }

    #[tokio::test]
    async fn test_read_range_into_buf_reads_after_seek() {
        let mut cursor = std::io::Cursor::new(b"0123456789".to_vec());
        seek_to(&mut cursor, 3).await.unwrap();
        let bytes = read_range_into_buf(&mut cursor, 4).await.unwrap();
        assert_eq!(bytes.as_ref(), b"3456");
    }
}
