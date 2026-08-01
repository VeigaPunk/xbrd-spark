//! xbrd-spark — pure L3 execution substrate for codex-spark under xbrd.
//!
//! Isolation without git worktrees. Unique spark-id → namespaced ephemeral dir.
//! Double-work is allowed; higher orchestrator (distiller/judge) collects + dedups.
//! Any CLI or delegated agent (labrat, mutation-tester, executor) can invoke.

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Instant;
use uuid::Uuid;

#[derive(Parser, Debug)]
#[command(name = "xbrd-spark", about = "Pure L3 codex-spark worker surface (no worktrees, double-work OK)")]
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

        /// Read-only preference (pass sandbox flags when possible).
        #[arg(long, default_value_t = false)]
        ro: bool,

        /// Timeout seconds (0 = no timeout). Soft; real enforcement is future work.
        #[arg(long, default_value_t = 120)]
        timeout: u64,

        /// Prefer direct `codex` over `xask --spark` (pure L3, min latency).
        #[arg(long, default_value_t = false)]
        direct: bool,

        /// Override root for spark dirs (default: $XDG_RUNTIME_DIR/xbrd-spark or /tmp/...).
        #[arg(long, env = "XBRD_SPARK_ROOT")]
        root: Option<PathBuf>,

        /// Keep artifacts after run (default true; gc later).
        #[arg(long, default_value_t = true)]
        keep: bool,

        /// Deterministic id from task+scope hash (optional early dedup). Default random for diversity.
        #[arg(long, default_value_t = false)]
        deterministic: bool,
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
    root: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct ResultJson {
    status: String,
    stdout: String,
    stderr: String,
    exit: Option<i32>,
    duration_ms: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct CollectRecord {
    spark_id: String,
    content_hash: String,
    status: String,
    result_path: String,
    artifacts: Vec<String>,
    provenance: Meta,
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

fn hash_str(s: &str) -> String {
    let mut h = Sha256::new();
    h.update(s.as_bytes());
    hex::encode(h.finalize())
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
    let host = dirs_home().join(".codex");
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
    // Preserve essential host vars only
    for k in ["PATH", "USER", "LANG", "LC_ALL", "TERM", "SSH_AUTH_SOCK", "DISPLAY"] {
        if let Ok(v) = env::var(k) {
            envm.insert(k.to_string(), v);
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

fn find_dispatcher(direct: bool) -> Result<(String, Vec<String>)> {
    // Prefer pure direct codex for L3 substrate (single source of flags).
    // xask is optional convenience for migration / loadout continuity.
    if !direct {
        if let Ok(p) = which::which("xask") {
            return Ok((
                p.to_string_lossy().into_owned(),
                vec!["--spark".into(), "--gs".into(), "codex".into()],
            ));
        }
    }
    let p = which::which("codex").context("neither xask nor codex found on PATH")?;
    Ok((
        p.to_string_lossy().into_owned(),
        vec![
            "exec".into(),
            "-m".into(),
            "gpt-5.3-codex-spark".into(),
            "-c".into(),
            "model_reasoning_effort=low".into(),
            "--ephemeral".into(),
            "--skip-git-repo-check".into(),
            "--color".into(),
            "never".into(),
            "--sandbox".into(),
            "danger-full-access".into(),
            "-c".into(),
            "approval_policy=never".into(),
        ],
    ))
}

fn run_spark(
    id: &str,
    task: &str,
    scope: Option<&Path>,
    ro: bool,
    _timeout: u64,
    direct: bool,
    root: &Path,
    keep: bool,
) -> Result<()> {
    let base = spark_dir(root, id);
    ensure_dirs(&base)?;
    seed_codex_home(&base.join("codex-home"))?;

    let task_hash = hash_str(task);
    fs::write(base.join("in/task.md"), task)?;

    if let Some(s) = scope {
        rsync_scope(s, &base.join("workspace"))?;
    }

    let (bin, mut args) = find_dispatcher(direct)?;
    args.push(task.to_string());

    let mut meta = Meta {
        spark_id: id.to_string(),
        started_at: chrono::Utc::now().to_rfc3339(),
        finished_at: None,
        duration_ms: None,
        model: "gpt-5.3-codex-spark".into(),
        cmdline: std::iter::once(bin.clone())
            .chain(args.iter().cloned())
            .collect(),
        status: "running".into(),
        exit_code: None,
        content_hash: None,
        task_hash,
        invoker: env::var("USER").unwrap_or_else(|_| "unknown".into()),
        scope: scope.map(|p| p.display().to_string()),
        ro,
        root: base.display().to_string(),
    };
    fs::write(
        base.join("meta.json"),
        serde_json::to_string_pretty(&meta)?,
    )?;

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
        .envs(&envm);

    let output = cmd.output().context("failed to spawn spark dispatcher")?;

    let duration_ms = start.elapsed().as_millis() as u64;
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    let exit = output.status.code();

    let status = if output.status.success() { "ok" } else { "fail" };

    let result = ResultJson {
        status: status.into(),
        stdout: stdout.clone(),
        stderr: stderr.clone(),
        exit,
        duration_ms,
    };
    // Atomic-ish write
    let tmp_result = base.join("out/result.json.tmp");
    fs::write(&tmp_result, serde_json::to_string_pretty(&result)?)?;
    fs::rename(&tmp_result, base.join("out/result.json"))?;

    fs::write(base.join("logs/stdout.log"), &stdout)?;
    fs::write(base.join("logs/stderr.log"), &stderr)?;

    let content_hash = hash_str(&format!("{}|{}|{}", status, stdout, stderr));

    meta.finished_at = Some(chrono::Utc::now().to_rfc3339());
    meta.duration_ms = Some(duration_ms);
    meta.status = status.into();
    meta.exit_code = exit;
    meta.content_hash = Some(content_hash.clone());
    let tmp_meta = base.join("meta.json.tmp");
    fs::write(&tmp_meta, serde_json::to_string_pretty(&meta)?)?;
    fs::rename(&tmp_meta, base.join("meta.json"))?;

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

    let record = CollectRecord {
        spark_id: id.to_string(),
        content_hash,
        status: status.into(),
        result_path: base.join("out/result.json").display().to_string(),
        artifacts: fs::read_dir(base.join("out/artifacts"))?
            .filter_map(|e| e.ok())
            .map(|e| e.path().display().to_string())
            .collect(),
        provenance: meta,
    };
    println!("{}", serde_json::to_string(&record)?);

    if !keep {
        let _ = fs::remove_dir_all(&base);
    }

    if status != "ok" {
        std::process::exit(exit.unwrap_or(1));
    }
    Ok(())
}

fn collect(ids: &[String], root: &Path) -> Result<()> {
    for id in ids {
        let base = spark_dir(root, id);
        let meta_path = base.join("meta.json");
        if !meta_path.is_file() {
            eprintln!("missing meta for {}", id);
            continue;
        }
        let meta: Meta = serde_json::from_str(&fs::read_to_string(&meta_path)?)?;
        let result_path = base.join("out/result.json");
        let content_hash = meta.content_hash.clone().unwrap_or_default();
        let record = CollectRecord {
            spark_id: id.clone(),
            content_hash,
            status: meta.status.clone(),
            result_path: result_path.display().to_string(),
            artifacts: fs::read_dir(base.join("out/artifacts"))
                .into_iter()
                .flatten()
                .filter_map(|e| e.ok())
                .map(|e| e.path().display().to_string())
                .collect(),
            provenance: meta,
        };
        println!("{}", serde_json::to_string(&record)?);
    }
    Ok(())
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
        let meta_path = p.join("meta.json");
        let age_ok = if meta_path.is_file() {
            if let Ok(m) = serde_json::from_str::<Meta>(&fs::read_to_string(&meta_path)?) {
                if let Ok(t) = chrono::DateTime::parse_from_rfc3339(&m.started_at) {
                    t.with_timezone(&chrono::Utc) < cutoff
                } else {
                    true
                }
            } else {
                true
            }
        } else {
            true
        };
        if age_ok {
            eprintln!("gc {}", p.display());
            let _ = fs::remove_dir_all(&p);
        }
    }
    Ok(())
}

fn status(id: &str, root: &Path) -> Result<()> {
    let base = spark_dir(root, id);
    let meta_path = base.join("meta.json");
    if !meta_path.is_file() {
        bail!("no such spark: {}", id);
    }
    let raw = fs::read_to_string(meta_path)?;
    println!("{}", raw);
    Ok(())
}

fn main() -> Result<()> {
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
            keep,
            deterministic,
        } => {
            let root = root.unwrap_or_else(default_root);
            fs::create_dir_all(&root)?;
            let task = resolve_task(task, task_file)?;
            let id = if deterministic {
                let scope_h = scope
                    .as_ref()
                    .map(|p| hash_str(&p.display().to_string()))
                    .unwrap_or_default();
                format!("sp-{}", &hash_str(&format!("{}|{}", task, scope_h))[..16])
            } else {
                id.unwrap_or_else(|| format!("sp-{}", Uuid::new_v4()))
            };
            run_spark(
                &id,
                &task,
                scope.as_deref(),
                ro,
                timeout,
                direct,
                &root,
                keep,
            )?;
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
    }
    Ok(())
}
