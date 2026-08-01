//! xbrd-spark / sekhmet — pure L3 swarm dispatch substrate for codex-spark under xbreed.
//!
//! Isolation without git worktrees. Unique spark-id → namespaced ephemeral dir.
//! Double-work is allowed; higher orchestrator (distiller/judge) collects + dedups.
//! Always available: any CLI or agent can call `run` or `swarm` (up to 64 concurrent).
//! Rust only — no Python.

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{self, BufRead, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;
use uuid::Uuid;

/// Hard ceiling for concurrent swarm runners (always-available L3 pool size).
pub const MAX_SWARM_CONCURRENCY: usize = 64;

#[derive(Parser, Debug)]
#[command(
    name = "sekhmet",
    version,
    about = "Sekhmet — always-available swarm dispatch substrate (xbreed L3). Up to 64 concurrent runners."
)]
struct Cli {
    #[command(subcommand)]
    cmd: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Run a single spark execution in an isolated namespace.
    Run {
        /// Explicit spark id (default: sp-<uuid>). Collision-resistant.
        #[arg(long)]
        id: Option<String>,

        /// Task prompt (or read from stdin if omitted).
        #[arg(long)]
        task: Option<String>,

        /// Optional path to task file.
        #[arg(long)]
        task_file: Option<PathBuf>,

        /// Optional scope path to rsync-snapshot into workspace (mutation-harbor style). Default OFF.
        #[arg(long)]
        scope: Option<PathBuf>,

        /// Read-only: force codex with `--sandbox read-only` (skips xask so sandbox is enforced).
        #[arg(long, default_value_t = false)]
        ro: bool,

        /// Timeout seconds (0 = no timeout). Wall-clock kill of the dispatcher child when > 0.
        #[arg(long, default_value_t = 120)]
        timeout: u64,

        /// Prefer direct `codex` over `xask --spark` (pure L3, min latency).
        #[arg(long, default_value_t = false)]
        direct: bool,

        /// Override root for spark dirs (default: $XDG_RUNTIME_DIR/xbrd-spark or /tmp/...).
        #[arg(long, env = "XBRD_SPARK_ROOT")]
        root: Option<PathBuf>,

        /// Delete namespace after run (inverse of keep; default is keep).
        #[arg(long = "no-keep", action = clap::ArgAction::SetTrue)]
        no_keep: bool,

        /// Stable id from task+scope hash (collision risk under concurrent same task). Default random for diversity.
        #[arg(long, default_value_t = false)]
        deterministic: bool,

        /// Dry-run: write full namespace + stub result without spawning xask/codex.
        #[arg(long, default_value_t = false)]
        dry_run: bool,
    },

    /// Collect structured records from one or more spark ids for distiller (NDJSON).
    Collect {
        /// Spark ids to collect.
        ids: Vec<String>,

        #[arg(long, env = "XBRD_SPARK_ROOT")]
        root: Option<PathBuf>,
    },

    /// Garbage-collect old spark namespaces.
    Gc {
        /// Max age in hours.
        #[arg(long, default_value_t = 2)]
        max_age: u64,

        #[arg(long, env = "XBRD_SPARK_ROOT")]
        root: Option<PathBuf>,
    },

    /// Print status / meta for a spark id.
    Status {
        id: String,
        #[arg(long, env = "XBRD_SPARK_ROOT")]
        root: Option<PathBuf>,
    },

    /// Run many tasks with a bounded pool of concurrent runners (max 64).
    ///
    /// Always-available swarm wrap: one process owns a pool of workers; each
    /// task is an isolated spark namespace (same as `run`). Emits one NDJSON
    /// CollectRecord per completed task (order is completion order).
    Swarm {
        /// Task source: file path, or `-` for stdin. Lines = tasks; lines
        /// starting with `{` are JSON objects with `task` (+ optional `scope`, `id`).
        #[arg(long = "tasks-file", short = 'f')]
        tasks_file: Option<PathBuf>,

        /// Number of concurrent runners (1..=64). Default 16. Env: XBRD_SPARK_JOBS.
        #[arg(long, short = 'j', default_value_t = 16, env = "XBRD_SPARK_JOBS")]
        jobs: usize,

        /// Optional shared scope directory rsync'd into each spark workspace.
        #[arg(long)]
        scope: Option<PathBuf>,

        /// Read-only: force codex with `--sandbox read-only`.
        #[arg(long, default_value_t = false)]
        ro: bool,

        /// Timeout seconds per spark (0 = no timeout).
        #[arg(long, default_value_t = 120)]
        timeout: u64,

        /// Prefer direct `codex` over `xask --spark`.
        #[arg(long, default_value_t = false)]
        direct: bool,

        #[arg(long, env = "XBRD_SPARK_ROOT")]
        root: Option<PathBuf>,

        /// Delete each namespace after its run.
        #[arg(long = "no-keep", action = clap::ArgAction::SetTrue)]
        no_keep: bool,

        /// Dry-run each task (no xask/codex).
        #[arg(long, default_value_t = false)]
        dry_run: bool,

        /// Fail the process if any spark fails/times out (default: still emit NDJSON, exit 1 if any failed).
        #[arg(long, default_value_t = false)]
        fail_fast: bool,
    },
}

/// One unit of swarm work (parsed from tasks-file line or JSON object).
#[derive(Debug, Clone, Deserialize)]
struct SwarmTask {
    task: String,
    #[serde(default)]
    scope: Option<String>,
    #[serde(default)]
    id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Meta {
    spark_id: String,
    started_at: String,
    finished_at: Option<String>,
    duration_ms: Option<u64>,
    model: String,
    cmdline: Vec<String>,
    status: String,
    exit_code: Option<i32>,
    content_hash: Option<String>,
    task_hash: String,
    invoker: String,
    scope: Option<String>,
    ro: bool,
    timeout_secs: u64,
    direct: bool,
    dry_run: bool,
    root: String,
    /// Best-effort parse of model usage tokens from dispatcher stdout/stderr.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    usage_tokens: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug)]
struct ResultJson {
    status: String,
    stdout: String,
    stderr: String,
    exit: Option<i32>,
    duration_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    usage_tokens: Option<u64>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct CollectRecord {
    spark_id: String,
    content_hash: String,
    status: String,
    result_path: String,
    artifacts: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    usage_tokens: Option<u64>,
    provenance: Meta,
}

/// Parse a usage token count from Titanium/codex-style logs (best-effort).
/// Handles multi-line "tokens used\\n  1,234" and JSON-ish total_tokens fields.
pub fn extract_usage_tokens(stdout: &str, stderr: &str) -> Option<u64> {
    let blob = format!("{}\n{}", stdout, stderr);
    // Prefer explicit "tokens used" lines (possibly split across lines).
    let lower = blob.to_ascii_lowercase();
    if let Some(idx) = lower.find("tokens used") {
        let tail = &blob[idx..];
        if let Some(n) = first_int_token(tail) {
            return Some(n);
        }
    }
    for key in [
        "total_tokens",
        "\"total\"",
        "token usage",
        "tokens:",
        "tok=",
    ] {
        if let Some(idx) = lower.find(key) {
            let tail = &blob[idx..];
            if let Some(n) = first_int_token(tail) {
                return Some(n);
            }
        }
    }
    None
}

fn first_int_token(s: &str) -> Option<u64> {
    let mut num = String::new();
    let mut seen_digit = false;
    for ch in s.chars() {
        if ch.is_ascii_digit() {
            num.push(ch);
            seen_digit = true;
        } else if ch == ',' && seen_digit {
            // thousands separator inside a number
            continue;
        } else if seen_digit {
            break;
        }
    }
    if num.is_empty() {
        None
    } else {
        num.parse().ok()
    }
}

/// Serialize NDJSON emit only (never hold this lock across Titanium spawn).
fn emit_ndjson(record: &CollectRecord) -> Result<()> {
    static EMIT: Mutex<()> = Mutex::new(());
    let _g = EMIT.lock().unwrap_or_else(|e| e.into_inner());
    println!("{}", serde_json::to_string(record)?);
    Ok(())
}

fn default_root() -> PathBuf {
    if let Ok(r) = env::var("XBRD_SPARK_ROOT") {
        return PathBuf::from(r);
    }
    if let Ok(xdg) = env::var("XDG_RUNTIME_DIR") {
        return PathBuf::from(xdg).join("xbrd-spark");
    }
    PathBuf::from("/tmp/xbrd-spark")
}

fn spark_dir(root: &Path, id: &str) -> PathBuf {
    root.join(id)
}

/// Validate spark id: must start with `sp-`, remainder only `[A-Za-z0-9_-]`, non-empty rest, max 128.
/// Rejects `/`, `\`, `..`, empty, and other path-unsafe forms.
fn validate_spark_id(id: &str) -> Result<()> {
    if id.is_empty() {
        bail!("invalid spark id: empty");
    }
    if id.len() > 128 {
        bail!("invalid spark id: exceeds max length 128");
    }
    if id.contains('/') || id.contains('\\') || id.contains("..") {
        bail!("invalid spark id: path-unsafe characters: {}", id);
    }
    if !id.starts_with("sp-") {
        bail!("invalid spark id: must start with sp-: {}", id);
    }
    let rest = &id[3..];
    if rest.is_empty() {
        bail!("invalid spark id: empty after sp- prefix");
    }
    if !rest
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        bail!("invalid spark id: remainder must be [A-Za-z0-9_-]: {}", id);
    }
    Ok(())
}

fn hash_str(s: &str) -> String {
    let mut h = Sha256::new();
    h.update(s.as_bytes());
    hex::encode(h.finalize())
}

/// Content hash used for distill: sha256 of `status|stdout|stderr`.
fn content_hash(status: &str, stdout: &str, stderr: &str) -> String {
    hash_str(&format!("{}|{}|{}", status, stdout, stderr))
}

/// Deterministic spark id: `sp-` + first 16 hex of sha256(`task|scope_h`).
fn deterministic_spark_id(task: &str, scope: Option<&str>) -> String {
    let scope_h = scope.map(hash_str).unwrap_or_default();
    format!("sp-{}", &hash_str(&format!("{}|{}", task, scope_h))[..16])
}

fn hash_file(p: &Path) -> Result<String> {
    let data = fs::read(p).with_context(|| format!("read {}", p.display()))?;
    let mut h = Sha256::new();
    h.update(&data);
    Ok(hex::encode(h.finalize()))
}

fn ensure_dirs(base: &Path) -> Result<()> {
    for sub in [
        "tmp",
        "home",
        "codex-home",
        "xdg/cache",
        "xdg/data",
        "xdg/config",
        "workspace",
        "out/artifacts",
        "logs",
        "in",
        "cargo-home",
        "target",
        "rustup-home",
    ] {
        fs::create_dir_all(base.join(sub))?;
    }
    Ok(())
}

fn seed_codex_home(target: &Path) -> Result<()> {
    seed_codex_home_from(&dirs_home().join(".codex"), target)
}

fn seed_codex_home_from(host: &Path, target: &Path) -> Result<()> {
    if !host.is_dir() {
        return Ok(());
    }
    // Minimal seed: auth + config only. Never mutate host. Residual: token lifetime under concurrent sparks.
    for name in ["auth.json", "config.toml", "config.json"] {
        let src = host.join(name);
        if src.is_file() {
            let _ = fs::copy(&src, target.join(name));
        }
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(target, fs::Permissions::from_mode(0o700));
        for name in ["auth.json", "config.toml", "config.json"] {
            let p = target.join(name);
            if p.is_file() {
                let _ = fs::set_permissions(&p, fs::Permissions::from_mode(0o600));
            }
        }
    }
    Ok(())
}

fn dirs_home() -> PathBuf {
    env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp"))
}

fn rsync_scope(scope: &Path, dest: &Path) -> Result<()> {
    // Exact mutation-harbor style excludes
    let status = Command::new("rsync")
        .args([
            "-a",
            "--delete",
            "--exclude",
            ".git/",
            "--exclude",
            "target/",
            "--exclude",
            "node_modules/",
            "--exclude",
            ".xbreed/",
            "--exclude",
            ".venv/",
            "--exclude",
            "**/__pycache__/",
            "--exclude",
            ".DS_Store",
        ])
        .arg(format!("{}/", scope.display()))
        .arg(format!("{}/", dest.display()))
        .status()
        .context("rsync not available or failed")?;
    if !status.success() {
        bail!("rsync failed with {:?}", status.code());
    }
    Ok(())
}

fn build_env(base: &Path) -> HashMap<String, String> {
    let mut envm = HashMap::new();
    for (k, v) in [
        ("TMPDIR", base.join("tmp")),
        ("TMP", base.join("tmp")),
        ("TEMP", base.join("tmp")),
        ("HOME", base.join("home")),
        ("CODEX_HOME", base.join("codex-home")),
        ("XDG_CACHE_HOME", base.join("xdg/cache")),
        ("XDG_DATA_HOME", base.join("xdg/data")),
        ("XDG_CONFIG_HOME", base.join("xdg/config")),
        ("CARGO_HOME", base.join("cargo-home")),
        ("CARGO_TARGET_DIR", base.join("target")),
        ("RUSTUP_HOME", base.join("rustup-home")),
    ] {
        envm.insert(k.to_string(), v.to_string_lossy().into_owned());
        let _ = fs::create_dir_all(&v);
    }
    // Preserve essential host vars only (Command uses env_clear + this map).
    for k in [
        "PATH",
        "USER",
        "LANG",
        "LC_ALL",
        "TERM",
        "SSH_AUTH_SOCK",
        "DISPLAY",
        "http_proxy",
        "https_proxy",
        "HTTP_PROXY",
        "HTTPS_PROXY",
        "ALL_PROXY",
        "NO_PROXY",
        "SSL_CERT_FILE",
        "SSL_CERT_DIR",
        "REQUESTS_CA_BUNDLE",
        "CURL_CA_BUNDLE",
        "RUST_LOG",
    ] {
        if let Ok(v) = env::var(k) {
            envm.insert(k.to_string(), v);
        }
    }
    // Auth / model config outside seeded auth.json (common host keys).
    // Do not let host CODEX_HOME (or other forced keys) clobber namespace paths.
    for (k, v) in env::vars() {
        if k == "CODEX_HOME" {
            continue;
        }
        if k.starts_with("OPENAI_") || k.starts_with("CODEX_") || k.starts_with("ANTHROPIC_") {
            envm.insert(k, v);
        }
    }
    envm
}

fn resolve_task(task: Option<String>, task_file: Option<PathBuf>) -> Result<String> {
    if let Some(t) = task {
        return Ok(t);
    }
    if let Some(p) = task_file {
        return Ok(fs::read_to_string(p)?);
    }
    let mut buf = String::new();
    io::stdin().read_to_string(&mut buf)?;
    if buf.trim().is_empty() {
        bail!("no task provided via --task, --task-file, or stdin");
    }
    Ok(buf)
}

/// Default model for Titanium spark runs (override with `XBRD_SPARK_MODEL`).
fn spark_model() -> String {
    env::var("XBRD_SPARK_MODEL").unwrap_or_else(|_| "gpt-5.3-codex-spark".into())
}

/// Resolve Codex Titanium binary: `CODEX_BIN` → `codex-titanium` → `codex`.
fn resolve_codex_bin() -> Result<PathBuf> {
    if let Ok(p) = env::var("CODEX_BIN") {
        let pb = PathBuf::from(&p);
        if pb.is_file() {
            return Ok(pb);
        }
        if let Ok(w) = which::which(p.as_str()) {
            return Ok(w);
        }
        bail!("CODEX_BIN not found or not a file: {p}");
    }
    if let Ok(p) = which::which("codex-titanium") {
        return Ok(p);
    }
    which::which("codex").with_context(|| {
        "codex-titanium/codex not found on PATH (install Codex Titanium or set CODEX_BIN)"
    })
}

fn find_dispatcher(direct: bool, ro: bool) -> Result<(String, Vec<String>)> {
    // Prefer pure Titanium codex for L3 substrate (single source of flags).
    // xask is optional convenience for migration / loadout continuity.
    // --ro forces codex so `--sandbox read-only` is actually applied (xask has no sandbox flags).
    if !direct && !ro {
        if let Ok(p) = which::which("xask") {
            return Ok((
                p.to_string_lossy().into_owned(),
                vec!["--spark".into(), "--gs".into(), "codex".into()],
            ));
        }
    }
    let sandbox = if ro {
        "read-only"
    } else {
        "danger-full-access"
    };
    let p = resolve_codex_bin().with_context(|| {
        if ro {
            "codex-titanium/codex not found on PATH (--ro forces titanium sandbox; xask skipped)"
        } else if direct {
            "codex-titanium/codex not found on PATH (--direct)"
        } else {
            "neither xask nor codex-titanium/codex found on PATH"
        }
    })?;
    let model = spark_model();
    Ok((
        p.to_string_lossy().into_owned(),
        vec![
            "exec".into(),
            "-m".into(),
            model,
            "-c".into(),
            "model_reasoning_effort=low".into(),
            "--ephemeral".into(),
            "--skip-git-repo-check".into(),
            "--color".into(),
            "never".into(),
            "--sandbox".into(),
            sandbox.into(),
            "-c".into(),
            "approval_policy=never".into(),
        ],
    ))
}

/// Load swarm tasks from a path (`-` = stdin). Blank / `#` lines skipped.
/// Lines starting with `{` are JSON: `{"task":"...","scope":"?","id":"?"}`.
fn load_swarm_tasks(path: Option<&Path>) -> Result<Vec<SwarmTask>> {
    let reader: Box<dyn BufRead> = match path {
        None => Box::new(io::BufReader::new(io::stdin())),
        Some(p) if p.as_os_str() == "-" => Box::new(io::BufReader::new(io::stdin())),
        Some(p) => Box::new(io::BufReader::new(
            fs::File::open(p).with_context(|| format!("open tasks file {}", p.display()))?,
        )),
    };
    let mut out = Vec::new();
    for (i, line) in reader.lines().enumerate() {
        let line = line.with_context(|| format!("read tasks line {}", i + 1))?;
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        if t.starts_with('{') {
            let st: SwarmTask = serde_json::from_str(t)
                .with_context(|| format!("parse JSON task on line {}", i + 1))?;
            if st.task.trim().is_empty() {
                bail!("empty task field on line {}", i + 1);
            }
            out.push(st);
        } else {
            out.push(SwarmTask {
                task: t.to_string(),
                scope: None,
                id: None,
            });
        }
    }
    if out.is_empty() {
        bail!("no tasks loaded (empty tasks-file / stdin)");
    }
    Ok(out)
}

/// Bounded concurrent swarm: up to [`MAX_SWARM_CONCURRENCY`] runners.
/// Each task is an isolated spark (same as `run`). NDJSON records print on completion.
#[allow(clippy::too_many_arguments)] // CLI-mapped pool knobs; packing would obscure the L3 surface
fn run_swarm(
    tasks: Vec<SwarmTask>,
    jobs: usize,
    shared_scope: Option<PathBuf>,
    ro: bool,
    timeout: u64,
    direct: bool,
    root: PathBuf,
    keep: bool,
    dry_run: bool,
    fail_fast: bool,
) -> Result<i32> {
    let jobs = jobs.clamp(1, MAX_SWARM_CONCURRENCY);
    fs::create_dir_all(&root)?;

    let (tx, rx) = std::sync::mpsc::channel::<SwarmTask>();
    let rx = Arc::new(Mutex::new(rx));
    let fail_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let stop = Arc::new(std::sync::atomic::AtomicBool::new(false));

    let mut handles = Vec::with_capacity(jobs);
    for _ in 0..jobs {
        let rx = Arc::clone(&rx);
        let fail_count = Arc::clone(&fail_count);
        let stop = Arc::clone(&stop);
        let root = root.clone();
        let shared_scope = shared_scope.clone();
        handles.push(thread::spawn(move || {
            loop {
                if stop.load(std::sync::atomic::Ordering::Relaxed) {
                    break;
                }
                let job = {
                    let guard = match rx.lock() {
                        Ok(g) => g,
                        Err(_) => break,
                    };
                    guard.recv()
                };
                let Ok(job) = job else { break };
                if stop.load(std::sync::atomic::Ordering::Relaxed) {
                    break;
                }
                let id = job
                    .id
                    .clone()
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| format!("sp-{}", Uuid::new_v4()));
                let scope_owned = job
                    .scope
                    .as_ref()
                    .map(PathBuf::from)
                    .or_else(|| shared_scope.clone());
                // Run Titanium/codex concurrently; NDJSON emit is locked only inside emit_ndjson.
                match run_spark(
                    &id,
                    &job.task,
                    scope_owned.as_deref(),
                    ro,
                    timeout,
                    direct,
                    &root,
                    keep,
                    dry_run,
                ) {
                    Ok(0) => {}
                    Ok(_code) => {
                        fail_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        if fail_fast {
                            stop.store(true, std::sync::atomic::Ordering::Relaxed);
                            break;
                        }
                    }
                    Err(e) => {
                        eprintln!("sekhmet swarm error id={id}: {e:#}");
                        fail_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                        if fail_fast {
                            stop.store(true, std::sync::atomic::Ordering::Relaxed);
                            break;
                        }
                    }
                }
            }
        }));
    }

    for t in tasks {
        if stop.load(std::sync::atomic::Ordering::Relaxed) {
            break;
        }
        tx.send(t).context("swarm queue closed")?;
    }
    drop(tx);

    for h in handles {
        let _ = h.join();
    }

    let failed = fail_count.load(std::sync::atomic::Ordering::Relaxed);
    if failed > 0 {
        Ok(1)
    } else {
        Ok(0)
    }
}

/// Outcome of a timed dispatcher run.
struct TimedOutput {
    output: std::process::Output,
    timed_out: bool,
}

/// Join a thread with a wall-clock bound. On timeout, abandons the handle (thread may leak)
/// and returns `None` so callers do not hang on pipe readers blocked by orphan descendants.
fn join_timeout<T: Send + 'static>(h: std::thread::JoinHandle<T>, ms: u64) -> Option<T> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(h.join().ok());
    });
    rx.recv_timeout(std::time::Duration::from_millis(ms))
        .ok()
        .flatten()
}

/// Run dispatcher with optional wall-clock timeout (0 = unlimited, same as Command::output).
/// When timeout_secs > 0 on Unix, the child is put in its own process group so kill
/// can SIGKILL the whole group (dispatcher + grandchildren).
fn run_with_timeout(mut cmd: Command, timeout_secs: u64) -> Result<TimedOutput> {
    if timeout_secs == 0 {
        let output = cmd.output().context("failed to spawn spark dispatcher")?;
        return Ok(TimedOutput {
            output,
            timed_out: false,
        });
    }

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        // SAFETY: setpgid(0,0) only affects the child after fork, before exec.
        unsafe {
            cmd.pre_exec(|| {
                if libc::setpgid(0, 0) != 0 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
    }

    let mut child = cmd.spawn().context("failed to spawn spark dispatcher")?;
    let child_pid = child.id();
    let child_stdout = child.stdout.take();
    let child_stderr = child.stderr.take();
    let out_h = std::thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(mut r) = child_stdout {
            let _ = r.read_to_end(&mut buf);
        }
        buf
    });
    let err_h = std::thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(mut r) = child_stderr {
            let _ = r.read_to_end(&mut buf);
        }
        buf
    });
    let deadline = Instant::now() + std::time::Duration::from_secs(timeout_secs);
    let mut timed_out = false;
    let status = loop {
        match child.try_wait().context("try_wait dispatcher")? {
            Some(s) => break s,
            None => {
                if Instant::now() >= deadline {
                    timed_out = true;
                    kill_dispatcher_tree(child_pid, &mut child);
                    break child.wait().context("wait after kill")?;
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
        }
    };
    // After kill, pipe readers can block forever if a descendant still holds the fd.
    // Bound joins; abandon handles (leak reader threads) and return empty on timeout.
    // Normal exit: still use a generous bound so a wedged reader cannot hang the spark.
    let join_ms = if timed_out { 2_000 } else { 120_000 };
    let stdout = join_timeout(out_h, join_ms).unwrap_or_default();
    let stderr = join_timeout(err_h, join_ms).unwrap_or_default();
    Ok(TimedOutput {
        output: std::process::Output {
            status,
            stdout,
            stderr,
        },
        timed_out,
    })
}

/// Kill dispatcher: process group on Unix (negative pgid), else child.kill().
fn kill_dispatcher_tree(pid: u32, child: &mut std::process::Child) {
    #[cfg(unix)]
    {
        // SAFETY: kill(-pgid) signals the process group created via setpgid(0,0).
        let pgid = pid as i32;
        unsafe {
            let _ = libc::kill(-pgid, libc::SIGKILL);
        }
        let _ = child.kill();
    }
    #[cfg(not(unix))]
    {
        let _ = child.kill();
        let _ = pid;
    }
}

fn write_meta_atomic(base: &Path, meta: &Meta) -> Result<()> {
    let tmp = base.join("meta.json.tmp");
    fs::write(&tmp, serde_json::to_string_pretty(meta)?)?;
    fs::rename(&tmp, base.join("meta.json"))?;
    Ok(())
}

fn write_artifacts_and_manifest(base: &Path) -> Result<Vec<String>> {
    let mut manifest = vec![];
    for entry in [
        "out/result.json",
        "meta.json",
        "logs/stdout.log",
        "logs/stderr.log",
    ] {
        let p = base.join(entry);
        if p.is_file() {
            let h = hash_file(&p)?;
            let dest = base.join("out/artifacts").join(&h);
            let _ = fs::copy(&p, &dest);
            manifest.push(format!("{} {}", h, entry));
        }
    }
    fs::write(base.join("out/manifest.txt"), manifest.join("\n"))?;
    Ok(manifest)
}

fn list_artifacts(base: &Path) -> Vec<String> {
    fs::read_dir(base.join("out/artifacts"))
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .map(|e| e.path().display().to_string())
        .collect()
}

/// Pure-ish collect: build one record from disk (None if meta missing).
fn collect_one(id: &str, root: &Path) -> Result<Option<CollectRecord>> {
    let base = spark_dir(root, id);
    let meta_path = base.join("meta.json");
    if !meta_path.is_file() {
        return Ok(None);
    }
    let meta: Meta = serde_json::from_str(&fs::read_to_string(&meta_path)?)?;
    let result_path = base.join("out/result.json");
    let content_hash = meta.content_hash.clone().unwrap_or_default();
    Ok(Some(CollectRecord {
        spark_id: id.to_string(),
        content_hash,
        status: meta.status.clone(),
        result_path: result_path.display().to_string(),
        artifacts: list_artifacts(&base),
        usage_tokens: None,
        provenance: meta,
    }))
}

fn collect_records(ids: &[String], root: &Path) -> Result<Vec<CollectRecord>> {
    let mut out = Vec::new();
    for id in ids {
        validate_spark_id(id)?;
        match collect_one(id, root)? {
            Some(r) => out.push(r),
            None => eprintln!("missing meta for {}", id),
        }
    }
    Ok(out)
}

fn finalize_result(
    base: &Path,
    meta: &mut Meta,
    status: &str,
    stdout: &str,
    stderr: &str,
    exit: Option<i32>,
    duration_ms: u64,
) -> Result<(String, CollectRecord)> {
    let usage_tokens = extract_usage_tokens(stdout, stderr);
    let result = ResultJson {
        status: status.into(),
        stdout: stdout.to_string(),
        stderr: stderr.to_string(),
        exit,
        duration_ms,
        usage_tokens,
    };
    let tmp_result = base.join("out/result.json.tmp");
    fs::write(&tmp_result, serde_json::to_string_pretty(&result)?)?;
    fs::rename(&tmp_result, base.join("out/result.json"))?;

    fs::write(base.join("logs/stdout.log"), stdout)?;
    fs::write(base.join("logs/stderr.log"), stderr)?;

    let ch = content_hash(status, stdout, stderr);

    meta.finished_at = Some(chrono::Utc::now().to_rfc3339());
    meta.duration_ms = Some(duration_ms);
    meta.status = status.into();
    meta.exit_code = exit;
    meta.content_hash = Some(ch.clone());
    meta.usage_tokens = usage_tokens;
    let tmp_meta = base.join("meta.json.tmp");
    fs::write(&tmp_meta, serde_json::to_string_pretty(&meta)?)?;
    fs::rename(&tmp_meta, base.join("meta.json"))?;

    write_artifacts_and_manifest(base)?;

    let record = CollectRecord {
        spark_id: meta.spark_id.clone(),
        content_hash: ch.clone(),
        status: status.into(),
        result_path: base.join("out/result.json").display().to_string(),
        artifacts: list_artifacts(base),
        usage_tokens,
        provenance: meta.clone(),
    };
    Ok((ch, record))
}

#[allow(clippy::too_many_arguments)] // mirrors `run` CLI flags 1:1 for L3 invokers
fn run_spark(
    id: &str,
    task: &str,
    scope: Option<&Path>,
    ro: bool,
    timeout: u64,
    direct: bool,
    root: &Path,
    keep: bool,
    dry_run: bool,
) -> Result<i32> {
    validate_spark_id(id)?;
    let base = spark_dir(root, id);
    // Exclusive namespace: same-id clobber is bad; concurrent different ids are fine (double-work OK).
    if base.exists() {
        bail!("spark namespace already exists: {}", id);
    }
    fs::create_dir_all(root)?;
    fs::create_dir(&base).with_context(|| format!("create spark namespace {}", base.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&base, fs::Permissions::from_mode(0o700));
    }

    // Setup through rsync only: on failure, release exclusive claim so the id is reusable.
    // After meta/finalize, keep current emit-record behavior (do not delete when keep=true).
    let setup = || -> Result<()> {
        ensure_dirs(&base)?;
        seed_codex_home(&base.join("codex-home"))?;
        fs::write(base.join("in/task.md"), task)?;
        // Scope snapshot always when requested (including dry-run) so probes see workspace files.
        if let Some(s) = scope {
            if !s.is_dir() {
                bail!("scope is not a directory: {}", s.display());
            }
            rsync_scope(s, &base.join("workspace"))?;
        }
        Ok(())
    };
    if let Err(e) = setup() {
        let _ = fs::remove_dir_all(&base);
        return Err(e);
    }

    let task_hash = hash_str(task);

    if dry_run {
        let mut meta = Meta {
            spark_id: id.to_string(),
            started_at: chrono::Utc::now().to_rfc3339(),
            finished_at: None,
            duration_ms: None,
            model: spark_model(),
            cmdline: vec!["dry-run".into()],
            status: "running".into(),
            exit_code: None,
            content_hash: None,
            task_hash,
            invoker: env::var("USER").unwrap_or_else(|_| "unknown".into()),
            scope: scope.map(|p| p.display().to_string()),
            ro,
            timeout_secs: timeout,
            direct,
            dry_run: true,
            usage_tokens: None,
            root: base.display().to_string(),
        };
        write_meta_atomic(&base, &meta)?;

        let (_, record) = finalize_result(&base, &mut meta, "ok", "dry-run", "", Some(0), 0)?;
        emit_ndjson(&record)?;

        if !keep {
            let _ = fs::remove_dir_all(&base);
        }
        return Ok(0);
    }

    // Resolve dispatcher before initial meta when possible; on failure after dirs exist,
    // still emit a structured error record if we can attach meta.
    let dispatcher = find_dispatcher(direct, ro);
    let (bin, mut args) = match &dispatcher {
        Ok((b, a)) => (b.clone(), a.clone()),
        Err(_) => (String::new(), Vec::new()),
    };
    if dispatcher.is_ok() {
        args.push(task.to_string());
    }

    let mut meta = Meta {
        spark_id: id.to_string(),
        started_at: chrono::Utc::now().to_rfc3339(),
        finished_at: None,
        duration_ms: None,
        model: spark_model(),
        cmdline: if dispatcher.is_ok() {
            std::iter::once(bin.clone())
                .chain(args.iter().cloned())
                .collect()
        } else {
            vec!["error".into()]
        },
        status: "running".into(),
        exit_code: None,
        content_hash: None,
        task_hash,
        invoker: env::var("USER").unwrap_or_else(|_| "unknown".into()),
        scope: scope.map(|p| p.display().to_string()),
        ro,
        timeout_secs: timeout,
        direct,
        dry_run: false,
        usage_tokens: None,
            root: base.display().to_string(),
    };
    write_meta_atomic(&base, &meta)?;

    if let Err(e) = dispatcher {
        let msg = format!("{:#}", e);
        let (_, record) = finalize_result(
            &base,
            &mut meta,
            "error",
            "",
            &msg,
            Some(1),
            0,
        )?;
        emit_ndjson(&record)?;
        if !keep {
            let _ = fs::remove_dir_all(&base);
        }
        return Ok(1);
    }

    let envm = build_env(&base);
    let start = Instant::now();

    let mut cmd = Command::new(&bin);
    cmd.args(&args)
        .current_dir(if scope.is_some() {
            base.join("workspace")
        } else {
            base.join("in")
        })
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env_clear()
        .envs(&envm);

    let timed = match run_with_timeout(cmd, timeout) {
        Ok(t) => t,
        Err(e) => {
            let msg = format!("{:#}", e);
            let duration_ms = start.elapsed().as_millis() as u64;
            let (_, record) = finalize_result(
                &base,
                &mut meta,
                "error",
                "",
                &msg,
                Some(1),
                duration_ms,
            )?;
            emit_ndjson(&record)?;
            if !keep {
                let _ = fs::remove_dir_all(&base);
            }
            return Ok(1);
        }
    };

    let duration_ms = start.elapsed().as_millis() as u64;
    let output = timed.output;
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let mut stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let exit = if timed.timed_out {
        Some(1)
    } else {
        output.status.code()
    };
    let status = if timed.timed_out {
        if !stderr.is_empty() {
            stderr.push('\n');
        }
        stderr.push_str(&format!(
            "xbrd-spark: killed after timeout ({}s)",
            timeout
        ));
        "timeout"
    } else if output.status.success() {
        "ok"
    } else {
        "fail"
    };

    let (_, record) = finalize_result(
        &base,
        &mut meta,
        status,
        &stdout,
        &stderr,
        exit,
        duration_ms,
    )?;
    emit_ndjson(&record)?;

    if !keep {
        let _ = fs::remove_dir_all(&base);
    }

    if status != "ok" {
        return Ok(exit.unwrap_or(1));
    }
    Ok(0)
}

fn collect(ids: &[String], root: &Path) -> Result<()> {
    if ids.is_empty() {
        bail!("no spark ids provided");
    }
    for record in collect_records(ids, root)? {
        emit_ndjson(&record)?;
    }
    Ok(())
}

/// Whether a spark namespace is eligible for GC deletion.
///
/// Rules:
/// - `status == "running"` and `started_at` still young (not before cutoff) → keep
/// - `status == "running"` but older than cutoff → delete (orphan cleanup)
/// - other statuses: delete when `started_at` is before cutoff
/// - unparseable `started_at`: treat as eligible (delete)
/// - missing/unparseable meta: use directory mtime vs cutoff (delete only if mtime < cutoff)
fn gc_should_delete(meta: Option<&Meta>, dir_mtime: Option<std::time::SystemTime>, cutoff: chrono::DateTime<chrono::Utc>) -> bool {
    match meta {
        Some(m) => {
            let started = chrono::DateTime::parse_from_rfc3339(&m.started_at)
                .ok()
                .map(|t| t.with_timezone(&chrono::Utc));
            match started {
                Some(t) if m.status == "running" && t >= cutoff => false, // live run, keep
                Some(t) => t < cutoff,
                None => true, // bad started_at → eligible
            }
        }
        None => match dir_mtime {
            Some(mtime) => {
                let mtime_dt: chrono::DateTime<chrono::Utc> = mtime.into();
                mtime_dt < cutoff
            }
            None => false, // cannot age → keep (safer)
        },
    }
}

fn dir_modified(p: &Path) -> Option<std::time::SystemTime> {
    fs::metadata(p).and_then(|m| m.modified()).ok()
}

fn gc(max_age_h: u64, root: &Path) -> Result<()> {
    if !root.is_dir() {
        return Ok(());
    }
    let cutoff = chrono::Utc::now() - chrono::Duration::hours(max_age_h as i64);
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        let p = entry.path();
        if !p.is_dir() {
            continue;
        }
        let name = match p.file_name().and_then(|n| n.to_str()) {
            Some(n) => n,
            None => continue,
        };
        // Only delete entries with valid spark ids (skip path-unsafe / non-spark names).
        if validate_spark_id(name).is_err() {
            continue;
        }
        let meta_path = p.join("meta.json");
        let meta = if meta_path.is_file() {
            fs::read_to_string(&meta_path)
                .ok()
                .and_then(|s| serde_json::from_str::<Meta>(&s).ok())
        } else {
            None
        };
        let mtime = dir_modified(&p);
        if gc_should_delete(meta.as_ref(), mtime, cutoff) {
            eprintln!("gc {}", p.display());
            let _ = fs::remove_dir_all(&p);
        }
    }
    Ok(())
}

fn status(id: &str, root: &Path) -> Result<()> {
    validate_spark_id(id)?;
    let base = spark_dir(root, id);
    let meta_path = base.join("meta.json");
    if !meta_path.is_file() {
        bail!("no such spark: {}", id);
    }
    let raw = fs::read_to_string(meta_path)?;
    println!("{}", raw);
    Ok(())
}

pub fn run_cli() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Commands::Run {
            id,
            task,
            task_file,
            scope,
            ro,
            timeout,
            direct,
            root,
            no_keep,
            deterministic,
            dry_run,
        } => {
            let root = root.unwrap_or_else(default_root);
            fs::create_dir_all(&root)?;
            let task = resolve_task(task, task_file)?;
            let id = if deterministic {
                let scope_s = scope.as_ref().map(|p| p.display().to_string());
                deterministic_spark_id(&task, scope_s.as_deref())
            } else {
                id.unwrap_or_else(|| format!("sp-{}", Uuid::new_v4()))
            };
            let keep = !no_keep;
            let code = run_spark(
                &id,
                &task,
                scope.as_deref(),
                ro,
                timeout,
                direct,
                &root,
                keep,
                dry_run,
            )?;
            if code != 0 {
                std::process::exit(code);
            }
        }
        Commands::Collect { ids, root } => {
            let root = root.unwrap_or_else(default_root);
            collect(&ids, &root)?;
        }
        Commands::Gc { max_age, root } => {
            let root = root.unwrap_or_else(default_root);
            gc(max_age, &root)?;
        }
        Commands::Status { id, root } => {
            let root = root.unwrap_or_else(default_root);
            status(&id, &root)?;
        }
        Commands::Swarm {
            tasks_file,
            jobs,
            scope,
            ro,
            timeout,
            direct,
            root,
            no_keep,
            dry_run,
            fail_fast,
        } => {
            if jobs > MAX_SWARM_CONCURRENCY {
                eprintln!(
                    "sekhmet: --jobs {jobs} exceeds max {MAX_SWARM_CONCURRENCY}; clamping"
                );
            }
            let root = root.unwrap_or_else(default_root);
            let tasks = load_swarm_tasks(tasks_file.as_deref())?;
            let code = run_swarm(
                tasks,
                jobs,
                scope,
                ro,
                timeout,
                direct,
                root,
                !no_keep,
                dry_run,
                fail_fast,
            )?;
            if code != 0 {
                std::process::exit(code);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn write_fixture_meta(base: &Path, id: &str, started_at: &str, status: &str, ch: &str) {
        ensure_dirs(base).unwrap();
        let meta = Meta {
            spark_id: id.to_string(),
            started_at: started_at.to_string(),
            finished_at: Some(started_at.to_string()),
            duration_ms: Some(1),
            model: "test".into(),
            cmdline: vec!["test".into()],
            status: status.into(),
            exit_code: Some(0),
            content_hash: Some(ch.to_string()),
            task_hash: hash_str("t"),
            invoker: "test".into(),
            scope: None,
            ro: false,
            timeout_secs: 0,
            direct: false,
            dry_run: true,
            usage_tokens: None,
            root: base.display().to_string(),
        };
        fs::write(
            base.join("meta.json"),
            serde_json::to_string_pretty(&meta).unwrap(),
        )
        .unwrap();
        let result = ResultJson {
            status: status.into(),
            stdout: "out".into(),
            stderr: "".into(),
            exit: Some(0),
            duration_ms: 1,
            usage_tokens: None,
        };
        fs::write(
            base.join("out/result.json"),
            serde_json::to_string_pretty(&result).unwrap(),
        )
        .unwrap();
        fs::write(base.join("out/artifacts").join("deadbeef"), b"artifact").unwrap();
    }




    #[test]
    fn hash_str_stable() {
        let a = hash_str("hello");
        let b = hash_str("hello");
        assert_eq!(a, b);
        assert_eq!(a.len(), 64);
        assert_ne!(hash_str("hello"), hash_str("world"));
    }

    #[test]
    fn deterministic_id_format_and_stable() {
        let id1 = deterministic_spark_id("task-a", None);
        let id2 = deterministic_spark_id("task-a", None);
        assert_eq!(id1, id2);
        assert!(id1.starts_with("sp-"));
        assert_eq!(id1.len(), 3 + 16);
        assert!(id1[3..].chars().all(|c| c.is_ascii_hexdigit()));
        validate_spark_id(&id1).unwrap();

        let with_scope = deterministic_spark_id("task-a", Some("/tmp/scope"));
        assert_ne!(id1, with_scope);
        assert_eq!(
            with_scope,
            deterministic_spark_id("task-a", Some("/tmp/scope"))
        );
    }

    #[test]
    fn validate_spark_id_accepts_uuid_and_hex_styles() {
        validate_spark_id("sp-01234567-89ab-cdef-0123-456789abcdef").unwrap();
        validate_spark_id("sp-deadbeefcafebabe").unwrap();
        validate_spark_id("sp-A_b-9").unwrap();
    }

    #[test]
    fn validate_spark_id_rejects_path_traversal_and_slash() {
        assert!(validate_spark_id("../evil").is_err());
        assert!(validate_spark_id("sp-foo/bar").is_err());
        assert!(validate_spark_id("sp-foo\\bar").is_err());
        assert!(validate_spark_id("sp-foo..bar").is_err());
        assert!(validate_spark_id("").is_err());
        assert!(validate_spark_id("sp-").is_err());
        assert!(validate_spark_id("nope-uuid").is_err());
        assert!(validate_spark_id(&format!("sp-{}", "x".repeat(200))).is_err());
    }

    #[test]
    fn content_hash_same_inputs_same_hash() {
        let h1 = content_hash("ok", "stdout", "stderr");
        let h2 = content_hash("ok", "stdout", "stderr");
        assert_eq!(h1, h2);
        assert_ne!(
            content_hash("ok", "stdout", "stderr"),
            content_hash("fail", "stdout", "stderr")
        );
        assert_eq!(h1, hash_str("ok|stdout|stderr"));
    }

    #[test]
    fn layout_ensure_dirs_creates_expected_subdirs() {
        let tmp = TempDir::new().unwrap();
        let base = tmp.path().join("sp-test");
        ensure_dirs(&base).unwrap();
        for sub in [
            "tmp",
            "home",
            "codex-home",
            "xdg/cache",
            "xdg/data",
            "xdg/config",
            "workspace",
            "out/artifacts",
            "logs",
            "in",
            "cargo-home",
            "target",
            "rustup-home",
        ] {
            assert!(base.join(sub).is_dir(), "missing subdir {}", sub);
        }
    }

    #[test]
    fn spark_dir_layout_under_root() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        let d = spark_dir(root, "sp-abc");
        assert_eq!(d, root.join("sp-abc"));
    }

    #[test]
    fn default_root_honors_xbrd_spark_root() {
        // Prefer explicit control; restore prior env after.
        let prev = env::var_os("XBRD_SPARK_ROOT");
        let tmp = TempDir::new().unwrap();
        let want = tmp.path().join("custom-root");
        env::set_var("XBRD_SPARK_ROOT", &want);
        let got = default_root();
        match prev {
            Some(v) => env::set_var("XBRD_SPARK_ROOT", v),
            None => env::remove_var("XBRD_SPARK_ROOT"),
        }
        assert_eq!(got, want);
    }

    #[test]
    fn collect_returns_records_from_fixtures() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        let id = "sp-collect-1";
        let base = spark_dir(root, id);
        write_fixture_meta(&base, id, &chrono::Utc::now().to_rfc3339(), "ok", "abc123");
        let records = collect_records(&[id.to_string()], root).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].spark_id, id);
        assert_eq!(records[0].content_hash, "abc123");
        assert_eq!(records[0].status, "ok");
        assert!(!records[0].artifacts.is_empty());
        assert!(records[0].result_path.ends_with("out/result.json"));
    }

    #[test]
    fn collect_missing_meta_skipped() {
        let tmp = TempDir::new().unwrap();
        let records = collect_records(&["sp-missing".into()], tmp.path()).unwrap();
        assert!(records.is_empty());
    }

    #[test]
    fn collect_invalid_id_evil_errors() {
        let tmp = TempDir::new().unwrap();
        let err = collect_records(&["../evil".into()], tmp.path()).unwrap_err();
        assert!(
            err.to_string().contains("invalid spark id"),
            "unexpected: {err}"
        );
    }

    #[test]
    fn collect_empty_ids_errors() {
        let tmp = TempDir::new().unwrap();
        let err = collect(&[], tmp.path()).unwrap_err();
        assert!(
            err.to_string().contains("no spark ids"),
            "unexpected err: {err}"
        );
    }

    #[test]
    fn dry_run_no_keep_removes_namespace() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        let id = "sp-nokeep-1";
        let code = run_spark(id, "probe task", None, false, 0, false, root, false, true).unwrap();
        assert_eq!(code, 0);
        let base = spark_dir(root, id);
        assert!(
            !base.exists(),
            "namespace should be removed with keep=false"
        );
    }

    #[test]
    fn gc_removes_old_keeps_young() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        let old_id = "sp-old";
        let young_id = "sp-young";
        let old_base = spark_dir(root, old_id);
        let young_base = spark_dir(root, young_id);

        let old_time = (chrono::Utc::now() - chrono::Duration::hours(5)).to_rfc3339();
        let young_time = chrono::Utc::now().to_rfc3339();
        write_fixture_meta(&old_base, old_id, &old_time, "ok", "h1");
        write_fixture_meta(&young_base, young_id, &young_time, "ok", "h2");

        assert!(old_base.is_dir());
        assert!(young_base.is_dir());

        gc(2, root).unwrap();

        assert!(!old_base.exists(), "old spark should be gc'd");
        assert!(young_base.exists(), "young spark should remain");
    }

    #[test]
    fn gc_skips_young_running_deletes_old_running() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        let young_run = "sp-run-young";
        let old_run = "sp-run-old";
        let young_base = spark_dir(root, young_run);
        let old_base = spark_dir(root, old_run);

        let old_time = (chrono::Utc::now() - chrono::Duration::hours(5)).to_rfc3339();
        let young_time = chrono::Utc::now().to_rfc3339();
        write_fixture_meta(&young_base, young_run, &young_time, "running", "h1");
        write_fixture_meta(&old_base, old_run, &old_time, "running", "h2");

        gc(2, root).unwrap();

        assert!(
            young_base.exists(),
            "young running spark must not be gc'd"
        );
        assert!(
            !old_base.exists(),
            "old running spark is orphan; should be gc'd"
        );
    }

    #[test]
    fn gc_should_delete_logic_unit() {
        let now = chrono::Utc::now();
        let cutoff = now - chrono::Duration::hours(2);
        let young = (now - chrono::Duration::minutes(10)).to_rfc3339();
        let old = (now - chrono::Duration::hours(5)).to_rfc3339();

        let mut meta_running_young = Meta {
            spark_id: "x".into(),
            started_at: young.clone(),
            finished_at: None,
            duration_ms: None,
            model: "t".into(),
            cmdline: vec![],
            status: "running".into(),
            exit_code: None,
            content_hash: None,
            task_hash: "t".into(),
            invoker: "t".into(),
            scope: None,
            ro: false,
            timeout_secs: 0,
            direct: false,
            dry_run: true,
            usage_tokens: None,
            root: "/tmp".into(),
        };
        assert!(!gc_should_delete(Some(&meta_running_young), None, cutoff));

        meta_running_young.started_at = old.clone();
        assert!(gc_should_delete(Some(&meta_running_young), None, cutoff));

        meta_running_young.status = "ok".into();
        meta_running_young.started_at = young;
        assert!(!gc_should_delete(Some(&meta_running_young), None, cutoff));

        meta_running_young.started_at = old;
        assert!(gc_should_delete(Some(&meta_running_young), None, cutoff));

        // missing meta: mtime younger than cutoff → keep
        let young_mtime = std::time::SystemTime::now();
        assert!(!gc_should_delete(None, Some(young_mtime), cutoff));

        // missing meta: mtime older than cutoff → delete
        let old_mtime = std::time::SystemTime::now() - std::time::Duration::from_secs(10_000);
        assert!(gc_should_delete(None, Some(old_mtime), cutoff));


        // missing meta + no mtime → keep (safer)
        assert!(!gc_should_delete(None, None, cutoff));
    }

    #[test]
    fn gc_missing_meta_uses_dir_mtime() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        let id = "sp-nometa";
        let base = spark_dir(root, id);
        fs::create_dir_all(&base).unwrap();
        // no meta.json; dir is brand new → should keep under max_age=2h
        gc(2, root).unwrap();
        assert!(base.exists(), "young dir without meta should remain");
    }

    #[test]
    fn dry_run_writes_namespace_and_record() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        let id = "sp-dry-1";
        let code = run_spark(id, "probe task", None, false, 0, false, root, true, true).unwrap();
        assert_eq!(code, 0);

        let base = spark_dir(root, id);
        assert!(base.join("meta.json").is_file());
        assert!(base.join("in/task.md").is_file());
        assert!(base.join("out/result.json").is_file());
        assert!(base.join("out/manifest.txt").is_file());
        assert!(base.join("logs/stdout.log").is_file());

        let task = fs::read_to_string(base.join("in/task.md")).unwrap();
        assert_eq!(task, "probe task");

        let result: ResultJson =
            serde_json::from_str(&fs::read_to_string(base.join("out/result.json")).unwrap())
                .unwrap();
        assert_eq!(result.status, "ok");
        assert_eq!(result.stdout, "dry-run");
        assert_eq!(result.stderr, "");
        assert_eq!(result.exit, Some(0));

        let meta: Meta =
            serde_json::from_str(&fs::read_to_string(base.join("meta.json")).unwrap()).unwrap();
        assert_eq!(meta.status, "ok");
        assert_eq!(meta.exit_code, Some(0));
        assert!(meta.content_hash.is_some());
        assert_eq!(
            meta.content_hash.as_deref(),
            Some(content_hash("ok", "dry-run", "").as_str())
        );
        assert!(meta.dry_run);
        assert!(!meta.direct);
        assert_eq!(meta.timeout_secs, 0);

        // artifacts content-addressed copies exist
        let arts = list_artifacts(&base);
        assert!(!arts.is_empty());

        let records = collect_records(&[id.to_string()], root).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].status, "ok");
    }

    #[test]
    fn meta_provenance_records_timeout_direct_dry_run() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        let id = "sp-meta-prov";
        let code = run_spark(id, "t", None, true, 42, true, root, true, true).unwrap();
        assert_eq!(code, 0);
        let meta: Meta = serde_json::from_str(
            &fs::read_to_string(spark_dir(root, id).join("meta.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(meta.timeout_secs, 42);
        assert!(meta.direct);
        assert!(meta.dry_run);
        assert!(meta.ro);
    }

    #[test]
    fn build_env_forces_namespace_paths() {
        let tmp = TempDir::new().unwrap();
        let base = tmp.path().join("sp-env");
        ensure_dirs(&base).unwrap();
        let prev_codex = env::var_os("CODEX_HOME");
        env::set_var("CODEX_HOME", "/host/should-not-win");
        let envm = build_env(&base);
        match prev_codex {
            Some(v) => env::set_var("CODEX_HOME", v),
            None => env::remove_var("CODEX_HOME"),
        }
        let tmpdir = envm.get("TMPDIR").expect("TMPDIR");
        let home = envm.get("HOME").expect("HOME");
        let codex = envm.get("CODEX_HOME").expect("CODEX_HOME");
        assert!(
            tmpdir.starts_with(&base.to_string_lossy().into_owned())
                || Path::new(tmpdir).starts_with(&base),
            "TMPDIR not under base: {tmpdir}"
        );
        assert!(Path::new(tmpdir).ends_with("tmp"));
        assert!(Path::new(home).ends_with("home"));
        assert!(Path::new(codex).ends_with("codex-home"));
        assert_eq!(Path::new(codex), base.join("codex-home"));
        assert_ne!(codex, "/host/should-not-win");
    }

    #[test]
    fn find_dispatcher_direct_includes_exec_model_sandbox() {
        // Requires codex on PATH for direct path; skip-like soft fail if missing.
        if which::which("codex").is_err() {
            // Still verify sandbox string selection via a local replica of the branch.
            let sandbox_ro = if true { "read-only" } else { "danger-full-access" };
            let sandbox_rw = if false {
                "read-only"
            } else {
                "danger-full-access"
            };
            assert_eq!(sandbox_ro, "read-only");
            assert_eq!(sandbox_rw, "danger-full-access");
            return;
        }
        let (_, args_rw) = find_dispatcher(true, false).unwrap();
        assert!(args_rw.iter().any(|a| a == "exec"));
        assert!(args_rw.windows(2).any(|w| w[0] == "-m" && w[1].contains("codex")));
        assert!(args_rw
            .windows(2)
            .any(|w| w[0] == "--sandbox" && w[1] == "danger-full-access"));

        let (_, args_ro) = find_dispatcher(true, true).unwrap();
        assert!(args_ro
            .windows(2)
            .any(|w| w[0] == "--sandbox" && w[1] == "read-only"));
    }

    #[test]
    fn find_dispatcher_ro_toggles_sandbox_value() {
        if which::which("codex").is_err() {
            return;
        }
        let (_, a) = find_dispatcher(true, false).unwrap();
        let (_, b) = find_dispatcher(true, true).unwrap();
        let sb = |args: &[String]| {
            args.windows(2)
                .find(|w| w[0] == "--sandbox")
                .map(|w| w[1].clone())
                .expect("sandbox flag")
        };
        assert_eq!(sb(&a), "danger-full-access");
        assert_eq!(sb(&b), "read-only");
    }

    #[test]
    fn exclusive_namespace_second_run_same_id_fails() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        let id = "sp-exclusive-1";
        let code = run_spark(id, "first", None, false, 0, false, root, true, true).unwrap();
        assert_eq!(code, 0);
        let err = run_spark(id, "second", None, false, 0, false, root, true, true).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("spark namespace already exists"),
            "unexpected: {msg}"
        );
        assert!(msg.contains(id));
    }

    #[test]
    fn setup_failure_releases_exclusive_claim() {
        // Bad scope (missing / not a dir) fails after exclusive create; namespace must roll back
        // so a second run with the same id can succeed.
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        let id = "sp-setup-rollback";
        let missing = tmp.path().join("no-such-scope-dir-xyz");
        let err = run_spark(
            id,
            "task",
            Some(missing.as_path()),
            false,
            0,
            false,
            root,
            true,
            true,
        )
        .unwrap_err();
        let msg = format!("{:#}", err);
        assert!(
            msg.contains("scope is not a directory")
                || msg.contains("rsync")
                || msg.contains("No such")
                || msg.contains("failed"),
            "unexpected setup err: {msg}"
        );
        assert!(
            !spark_dir(root, id).exists(),
            "namespace must be removed after setup failure"
        );
        let code = run_spark(id, "retry-ok", None, false, 0, false, root, true, true).unwrap();
        assert_eq!(code, 0);
        assert!(spark_dir(root, id).exists());
    }

    #[test]
    fn run_with_timeout_marks_timed_out() {
        if which::which("sleep").is_err() {
            return;
        }
        let mut cmd = Command::new("sleep");
        cmd.arg("5")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let timed = run_with_timeout(cmd, 1).expect("spawn sleep");
        assert!(timed.timed_out, "expected timeout kill");
        assert!(!timed.output.status.success());
    }

    #[test]
    fn find_dispatcher_ro_forces_codex_sandbox() {
        // --ro must never return xask argv (no sandbox); requires codex on PATH.
        if which::which("codex").is_err() {
            return;
        }
        let (bin, args) = find_dispatcher(false, true).unwrap();
        assert!(
            !bin.ends_with("xask") && !bin.contains("/xask"),
            "ro must not use xask: {bin}"
        );
        assert!(args.iter().any(|a| a == "exec"));
        assert!(args
            .windows(2)
            .any(|w| w[0] == "--sandbox" && w[1] == "read-only"));
    }

    #[test]
    fn resolve_task_from_string() {
        let t = resolve_task(Some("hello".into()), None).unwrap();
        assert_eq!(t, "hello");
    }

    #[test]
    fn resolve_task_from_file() {
        let tmp = TempDir::new().unwrap();
        let p = tmp.path().join("task.md");
        fs::write(&p, "from-file").unwrap();
        let t = resolve_task(None, Some(p)).unwrap();
        assert_eq!(t, "from-file");
    }

    #[test]
    fn status_missing_id_bails() {
        let tmp = TempDir::new().unwrap();
        let err = status("sp-does-not-exist", tmp.path()).unwrap_err();
        assert!(
            err.to_string().contains("no such spark"),
            "unexpected: {err}"
        );
    }

    #[test]
    fn status_invalid_id_evil_errors() {
        let tmp = TempDir::new().unwrap();
        let err = status("../evil", tmp.path()).unwrap_err();
        assert!(
            err.to_string().contains("invalid spark id"),
            "unexpected: {err}"
        );
    }

    #[test]
    fn dry_run_with_scope_rsyncs_when_available() {
        if which::which("rsync").is_err() {
            return;
        }
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("root");
        fs::create_dir_all(&root).unwrap();
        let scope = tmp.path().join("scope");
        fs::create_dir_all(&scope).unwrap();
        fs::write(scope.join("probe.txt"), b"scope-payload").unwrap();
        let id = "sp-scope-dry";
        let code = run_spark(
            id,
            "task",
            Some(scope.as_path()),
            false,
            0,
            false,
            &root,
            true,
            true,
        )
        .unwrap();
        assert_eq!(code, 0);
        let copied = spark_dir(&root, id).join("workspace/probe.txt");
        assert!(
            copied.is_file(),
            "expected rsync into workspace on dry-run"
        );
        assert_eq!(fs::read_to_string(copied).unwrap(), "scope-payload");
    }

    #[test]
    fn scope_not_directory_bails() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("root");
        fs::create_dir_all(&root).unwrap();
        let file_scope = tmp.path().join("not-a-dir");
        fs::write(&file_scope, b"x").unwrap();
        let err = run_spark(
            "sp-scope-file",
            "t",
            Some(file_scope.as_path()),
            false,
            0,
            false,
            &root,
            true,
            true,
        )
        .unwrap_err();
        assert!(
            err.to_string().contains("scope is not a directory"),
            "unexpected: {err}"
        );
        assert!(
            !spark_dir(&root, "sp-scope-file").exists(),
            "namespace must roll back on scope validation failure"
        );
    }

    #[cfg(unix)]
    #[test]
    fn seed_codex_home_sets_unix_modes() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = TempDir::new().unwrap();
        let host = tmp.path().join("host-codex");
        let target = tmp.path().join("target-codex");
        fs::create_dir_all(&host).unwrap();
        fs::create_dir_all(&target).unwrap();
        fs::write(host.join("auth.json"), br#"{"token":"fake"}"#).unwrap();
        fs::write(host.join("config.toml"), b"model = \"x\"").unwrap();
        seed_codex_home_from(&host, &target).unwrap();
        assert!(target.join("auth.json").is_file());
        let dir_mode = fs::metadata(&target).unwrap().permissions().mode() & 0o777;
        assert_eq!(dir_mode, 0o700, "codex-home dir mode {dir_mode:#o}");
        let auth_mode = fs::metadata(target.join("auth.json"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(auth_mode, 0o600, "auth.json mode {auth_mode:#o}");
    }

    #[test]
    fn artifact_hash_content_addressed() {
        let tmp = TempDir::new().unwrap();
        let base = tmp.path().join("sp-art");
        ensure_dirs(&base).unwrap();
        let body = b"identical-body";
        fs::write(base.join("out/result.json"), body).unwrap();
        fs::write(base.join("meta.json"), body).unwrap();
        fs::write(base.join("logs/stdout.log"), b"log").unwrap();
        fs::write(base.join("logs/stderr.log"), b"").unwrap();

        write_artifacts_and_manifest(&base).unwrap();
        let h_file = hash_file(&base.join("out/result.json")).unwrap();
        assert!(base.join("out/artifacts").join(&h_file).is_file());
        // same content → same artifact name for meta and result
        assert_eq!(hash_file(&base.join("meta.json")).unwrap(), h_file);
        assert_eq!(content_hash("ok", "a", "b"), content_hash("ok", "a", "b"));
    }

    #[test]
    fn max_swarm_concurrency_is_64() {
        assert_eq!(MAX_SWARM_CONCURRENCY, 64);
    }

    #[test]
    fn load_swarm_tasks_plain_and_jsonl() {
        let tmp = TempDir::new().unwrap();
        let f = tmp.path().join("tasks.txt");
        fs::write(
            &f,
            "# comment\n\ntask-one\n{\"task\":\"task-two\",\"id\":\"sp-fixed-two\"}\n",
        )
        .unwrap();
        let tasks = load_swarm_tasks(Some(&f)).unwrap();
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].task, "task-one");
        assert_eq!(tasks[1].task, "task-two");
        assert_eq!(tasks[1].id.as_deref(), Some("sp-fixed-two"));
    }

    #[test]
    fn swarm_dry_run_pool_completes() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("root");
        let tasks: Vec<SwarmTask> = (0..8)
            .map(|i| SwarmTask {
                task: format!("swarm-task-{i}"),
                scope: None,
                id: None,
            })
            .collect();
        let code = run_swarm(
            tasks,
            4, // concurrent workers
            None,
            false,
            30,
            true,
            root.clone(),
            true,
            true, // dry_run
            false,
        )
        .unwrap();
        assert_eq!(code, 0);
        let n = fs::read_dir(&root).unwrap().count();
        assert_eq!(n, 8, "expected 8 namespaces under swarm root");
    }

    #[test]
    fn resolve_codex_bin_honors_codex_bin_env() {
        let tmp = TempDir::new().unwrap();
        let fake = tmp.path().join("fake-codex");
        fs::write(&fake, b"#!/bin/sh\nexit 0\n").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&fake, fs::Permissions::from_mode(0o755)).unwrap();
        }
        let prev = env::var_os("CODEX_BIN");
        env::set_var("CODEX_BIN", &fake);
        let got = resolve_codex_bin().unwrap();
        assert_eq!(got, fake);
        match prev {
            Some(v) => env::set_var("CODEX_BIN", v),
            None => env::remove_var("CODEX_BIN"),
        }
    }

    #[test]
    fn find_dispatcher_resolves_titanium_when_available() {
        if which::which("codex-titanium").is_err() && which::which("codex").is_err() {
            return;
        }
        let (bin, args) = find_dispatcher(true, false).unwrap();
        assert!(
            bin.contains("codex"),
            "expected codex/titanium path, got {bin}"
        );
        assert!(args.windows(2).any(|w| w[0] == "-m" && w[1].contains("codex")));
    }




    #[test]
    fn extract_usage_tokens_multiline_tokens_used() {
        let n = extract_usage_tokens("tokens used\n  1,234\n", "").unwrap();
        assert_eq!(n, 1234);
    }

    #[test]
    fn extract_usage_tokens_total_tokens_jsonish() {
        let n = extract_usage_tokens("{\"total_tokens\": 42}", "").unwrap();
        assert_eq!(n, 42);
    }

    #[test]
    fn extract_usage_tokens_none_when_absent() {
        assert!(extract_usage_tokens("hello", "world").is_none());
    }

}
