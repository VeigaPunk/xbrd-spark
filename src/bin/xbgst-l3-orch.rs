//! xbgst L3 orch — 120m autonomous sekhmet waves under xbgst ship rules.
//!
//! - Up to 64 concurrent Titanium workers (ChatGPT OAuth; luna + service_tier=fast)
//! - Task kinds: issue analysis, review, polish, improve (xbgst roles)
//! - Strict-improvement ship: gates green + tree changed → commit + push `main`
//! - Tmp auto-clean after every wave (`--no-keep`, gc, purge /tmp sekhmet-*)
//!
//! Language: Rust only (xbgst lock).

use anyhow::{bail, Context, Result};
use clap::Parser;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[derive(Parser, Debug)]
#[command(
    name = "xbgst-l3-orch",
    about = "xbgst L3 sekhmet orch: issues + review/polish/improve; ship on strict improvement"
)]
struct Cli {
    #[arg(long, default_value_t = 120)]
    duration_mins: u64,

    #[arg(long, short = 'j', default_value_t = 64)]
    jobs: usize,

    /// gh issues JSON array.
    #[arg(long)]
    issues: PathBuf,

    /// Directory of local git repos to review/polish (one path per line), optional.
    #[arg(long)]
    repos_file: Option<PathBuf>,

    #[arg(long)]
    out: PathBuf,

    #[arg(long, default_value = "gpt-5.3-codex-spark", env = "XBRD_SPARK_MODEL")]
    model: String,

    #[arg(long, default_value = "fast", env = "XBRD_SPARK_SERVICE_TIER")]
    service_tier: String,

    #[arg(long, default_value_t = 8)]
    pause_secs: u64,

    #[arg(long)]
    tasks_per_wave: Option<usize>,

    /// Allow git commit+push main when gates pass and tree improved.
    #[arg(long, default_value_t = true)]
    ship: bool,

    /// Disable git push (commit only).
    #[arg(long, default_value_t = false)]
    no_push: bool,

    #[arg(long, default_value_t = false)]
    dry_run: bool,
}

#[derive(Debug, Clone, Deserialize)]
struct GhRepo {
    #[serde(rename = "nameWithOwner")]
    name_with_owner: Option<String>,
    name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct GhIssue {
    number: u64,
    title: String,
    url: Option<String>,
    body: Option<String>,
    repository: Option<GhRepo>,
}

#[derive(Debug, Clone, Serialize)]
struct Issue {
    repo: String,
    number: u64,
    title: String,
    url: String,
    body: String,
}

#[derive(Debug, Clone)]
struct LocalRepo {
    path: PathBuf,
    name: String,
}

// LocalRepo already Clone via PathBuf/String.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WaveKind {
    Issues,
    Review,
    Polish,
    Improve,
}

impl WaveKind {
    fn cycle(wave: u64) -> Self {
        match wave % 4 {
            1 => Self::Issues,
            2 => Self::Review,
            3 => Self::Polish,
            _ => Self::Improve,
        }
    }
    fn as_str(self) -> &'static str {
        match self {
            Self::Issues => "issues",
            Self::Review => "review",
            Self::Polish => "polish",
            Self::Improve => "improve",
        }
    }
}

#[derive(Serialize)]
struct WaveTelemetry {
    wave: u64,
    kind: String,
    started_at: String,
    finished_at: String,
    wall_secs: f64,
    tasks: usize,
    ok: usize,
    fail: usize,
    timeout: usize,
    error: usize,
    usage_tokens_sum: u64,
    usage_tokens_n: usize,
    sekhmet_exit: i32,
    root: String,
    model: String,
    service_tier: String,
    scope: Option<String>,
    ship: Option<ShipRecord>,
}

#[derive(Serialize, Clone)]
struct ShipRecord {
    repo: String,
    approved: bool,
    reason: String,
    commit: Option<String>,
    pushed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    push_err: Option<String>,
    dirty_before: bool,
    dirty_after: bool,
    gate_ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    gate_detail: Option<String>,
}

const ROLES: &[&str] = &[
    "labrat",
    "scout",
    "reviewer",
    "executor",
    "critic",
    "connector",
    "sentinel",
    "distiller",
    "simplifier",
    "mutation-tester",
    "the-revenger",
    "scribe",
];

fn main() {
    if let Err(e) = run() {
        eprintln!("xbgst-l3-orch: {e:#}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    if !(1..=64).contains(&cli.jobs) {
        bail!("--jobs must be 1..=64, got {}", cli.jobs);
    }
    let tasks_per_wave = cli.tasks_per_wave.unwrap_or(cli.jobs).clamp(1, 64);

    fs::create_dir_all(&cli.out)?;
    let telem = cli.out.join("telemetry");
    let waves_dir = telem.join("waves");
    fs::create_dir_all(&waves_dir)?;
    let tasks_dir = cli.out.join("tasks");
    fs::create_dir_all(&tasks_dir)?;
    let ships_path = telem.join("ships.ndjson");

    let issues = load_issues(&cli.issues)?;
    write_json(
        &cli.out.join("issues_index.json"),
        &issues
            .iter()
            .map(|i| {
                json!({
                    "repo": i.repo, "number": i.number, "title": i.title, "url": i.url
                })
            })
            .collect::<Vec<_>>(),
    )?;

    let repos = load_repos(cli.repos_file.as_deref())?;
    write_json(
        &cli.out.join("repos_index.json"),
        &repos
            .iter()
            .map(|r| json!({"name": r.name, "path": r.path.display().to_string()}))
            .collect::<Vec<_>>(),
    )?;

    if issues.is_empty() && repos.is_empty() {
        bail!("need at least issues or local repos");
    }

    let sekhmet = which_sekhmet()?;
    let deadline = Instant::now() + Duration::from_secs(cli.duration_mins.saturating_mul(60));
    let orch_start = Instant::now();

    write_json(
        &cli.out.join("session.json"),
        &json!({
            "orch": "xbgst-l3-orch",
            "started_at": chrono_now(),
            "duration_mins": cli.duration_mins,
            "jobs": cli.jobs,
            "tasks_per_wave": tasks_per_wave,
            "model": cli.model,
            "service_tier": cli.service_tier,
            "auth": "chatgpt-oauth",
            "ship": cli.ship,
            "no_push": cli.no_push,
            "issues_n": issues.len(),
            "repos_n": repos.len(),
            "sekhmet": sekhmet.display().to_string(),
            "out": cli.out.display().to_string(),
            "xbgst": {
                "connector_every_round": true,
                "ship_on_strict_improvement": true,
                "direct_to_main": true,
                "language": "rust-only"
            }
        }),
    )?;

    let mut wave: u64 = 0;
    let mut grand_ok = 0usize;
    let mut grand_fail = 0usize;
    let mut grand_tokens: u64 = 0;
    let mut grand_tasks = 0usize;
    let mut grand_ships = 0usize;
    let waves_ndjson = telem.join("waves.ndjson");
    let mut waves_file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&waves_ndjson)?;
    let mut ships_file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&ships_path)?;

    eprintln!(
        "xbgst-l3-orch: START {}m j={} issues={} repos={} model={} tier={} ship={} out={}",
        cli.duration_mins,
        cli.jobs,
        issues.len(),
        repos.len(),
        cli.model,
        cli.service_tier,
        cli.ship,
        cli.out.display()
    );

    while Instant::now() < deadline {
        wave += 1;
        if deadline.saturating_duration_since(Instant::now()) < Duration::from_secs(45) {
            eprintln!("xbgst-l3-orch: budget nearly exhausted; halt");
            break;
        }

        let kind = WaveKind::cycle(wave);
        let scope_repo = pick_scope_repo(&repos, wave, kind);
        let scope_path = scope_repo.as_ref().map(|r| r.path.clone());

        let dirty_before = scope_path
            .as_ref()
            .map(|p| git_dirty(p).unwrap_or(false))
            .unwrap_or(false);

        let (task_lines, _roles) =
            build_wave_tasks(&issues, &repos, kind, wave, tasks_per_wave, scope_repo.as_ref());
        let tasks_path = tasks_dir.join(format!("wave-{wave:04}-{}.txt", kind.as_str()));
        fs::write(&tasks_path, task_lines.join("\n") + "\n")?;

        let root = std::env::temp_dir().join(format!(
            "sekhmet-orch-w{wave}-{}",
            &uuid_simple()[..10]
        ));
        fs::create_dir_all(&root)?;
        let ndjson_out = waves_dir.join(format!("wave-{wave:04}.ndjson"));
        let stderr_out = waves_dir.join(format!("wave-{wave:04}.stderr.log"));

        let started_at = chrono_now();
        let wstart = Instant::now();

        let mut cmd = Command::new(&sekhmet);
        cmd.arg("swarm")
            .arg("--direct")
            .arg("-j")
            .arg(cli.jobs.to_string())
            .arg("--timeout")
            .arg("180")
            .arg("--no-keep")
            .arg("-f")
            .arg(&tasks_path)
            .arg("--root")
            .arg(&root)
            .env("XBRD_SPARK_MODEL", &cli.model)
            .env("XBRD_SPARK_FALLBACK_MODEL", "none")
            .env("XBRD_SPARK_SERVICE_TIER", &cli.service_tier)
            .env("XBRD_SPARK_ROOT", &root)
            .stdout(Stdio::from(fs::File::create(&ndjson_out)?))
            .stderr(Stdio::from(fs::File::create(&stderr_out)?));
        if let Some(ref sp) = scope_path {
            // Mutation-capable harbor for polish/improve/review on real trees
            cmd.arg("--scope").arg(sp);
        }
        // Issue analysis waves stay RO-friendly without scope
        if kind == WaveKind::Issues {
            cmd.arg("--ro");
        }
        if cli.dry_run {
            cmd.arg("--dry-run");
        }

        eprintln!(
            "xbgst-l3-orch: wave {wave} kind={} tasks={} scope={}",
            kind.as_str(),
            task_lines.len(),
            scope_path
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "-".into())
        );

        let status = cmd.status().context("sekhmet swarm")?;
        let sekhmet_exit = status.code().unwrap_or(1);
        let wall = wstart.elapsed().as_secs_f64();
        let finished_at = chrono_now();
        let stats = summarize_ndjson(&ndjson_out)?;
        grand_ok += stats.ok;
        grand_fail += stats.fail + stats.timeout + stats.error;
        grand_tokens += stats.usage_tokens_sum;
        grand_tasks += stats.tasks;

        // xbgst ship: only scoped waves that can mutate
        let mut ship: Option<ShipRecord> = None;
        if cli.ship && !cli.dry_run {
            if let Some(ref repo) = scope_repo {
                if matches!(kind, WaveKind::Review | WaveKind::Polish | WaveKind::Improve) {
                    match judge_and_ship(repo, dirty_before, kind, !cli.no_push) {
                        Ok(rec) => {
                            if rec.approved && rec.commit.is_some() {
                                grand_ships += 1;
                            }
                            writeln!(ships_file, "{}", serde_json::to_string(&rec)?)?;
                            ships_file.flush()?;
                            ship = Some(rec);
                        }
                        Err(e) => {
                            eprintln!("xbgst-l3-orch: ship error on {}: {e:#}", repo.name);
                        }
                    }
                }
            }
        }

        let wt = WaveTelemetry {
            wave,
            kind: kind.as_str().into(),
            started_at,
            finished_at,
            wall_secs: wall,
            tasks: stats.tasks,
            ok: stats.ok,
            fail: stats.fail,
            timeout: stats.timeout,
            error: stats.error,
            usage_tokens_sum: stats.usage_tokens_sum,
            usage_tokens_n: stats.usage_tokens_n,
            sekhmet_exit,
            root: root.display().to_string(),
            model: cli.model.clone(),
            service_tier: cli.service_tier.clone(),
            scope: scope_path.map(|p| p.display().to_string()),
            ship: ship.clone(),
        };
        writeln!(waves_file, "{}", serde_json::to_string(&wt)?)?;
        waves_file.flush()?;

        // Tmp never blocks
        let _ = Command::new(&sekhmet)
            .args(["gc", "--max-age", "0", "--root"])
            .arg(&root)
            .status();
        let _ = fs::remove_dir_all(&root);
        clean_tmp_globs()?;

        write_summary(&OrchSummary {
            telem: &telem,
            waves: wave,
            wall_secs: orch_start.elapsed().as_secs_f64(),
            tasks: grand_tasks,
            ok: grand_ok,
            fail: grand_fail,
            tokens: grand_tokens,
            ships: grand_ships,
            cli: &cli,
            issues_n: issues.len(),
            repos_n: repos.len(),
        })?;

        eprintln!(
            "xbgst-l3-orch: wave {wave} done kind={} exit={sekhmet_exit} ok={} fail={} tok={} wall={wall:.1}s ships={grand_ships}",
            kind.as_str(),
            stats.ok,
            stats.fail + stats.timeout + stats.error,
            stats.usage_tokens_sum
        );

        if Instant::now() + Duration::from_secs(cli.pause_secs) >= deadline {
            break;
        }
        std::thread::sleep(Duration::from_secs(cli.pause_secs));
    }

    clean_tmp_globs()?;
    write_summary(&OrchSummary {
        telem: &telem,
        waves: wave,
        wall_secs: orch_start.elapsed().as_secs_f64(),
        tasks: grand_tasks,
        ok: grand_ok,
        fail: grand_fail,
        tokens: grand_tokens,
        ships: grand_ships,
        cli: &cli,
        issues_n: issues.len(),
        repos_n: repos.len(),
    })?;
    fs::write(
        cli.out.join("DONE"),
        format!(
            "finished_at={}\nwaves={wave}\ntasks={grand_tasks}\nok={grand_ok}\nfail={grand_fail}\ntokens={grand_tokens}\nships={grand_ships}\nwall_secs={:.1}\n",
            chrono_now(),
            orch_start.elapsed().as_secs_f64()
        ),
    )?;
    eprintln!(
        "xbgst-l3-orch: COMPLETE waves={wave} tasks={grand_tasks} ok={grand_ok} tokens={grand_tokens} ships={grand_ships}"
    );
    Ok(())
}

/// xbgst judge: APPROVED only on strict improvement (dirty delta + green gates).
fn judge_and_ship(
    repo: &LocalRepo,
    dirty_before: bool,
    kind: WaveKind,
    do_push: bool,
) -> Result<ShipRecord> {
    // Drop titanium/noise artifacts so gates see real project state.
    sanitize_worktree(&repo.path);

    let dirty_after = git_dirty(&repo.path)?;
    let (gate_ok, gate_detail) = run_gates(&repo.path)?;

    // Strict improvement: real diff vs HEAD that gates accept.
    let has_diff = git_has_diff_vs_head(&repo.path)?;
    let improved = has_diff && gate_ok;

    if !improved {
        let reason = if !has_diff {
            "BLOCKED: no tree change vs HEAD (no strict improvement)".into()
        } else {
            format!(
                "BLOCKED: gates failed after mutation ({})",
                gate_detail.as_deref().unwrap_or("unknown")
            )
        };
        return Ok(ShipRecord {
            repo: repo.name.clone(),
            approved: false,
            reason,
            commit: None,
            pushed: false,
            push_err: None,
            dirty_before,
            dirty_after,
            gate_ok,
            gate_detail,
        });
    }

    // Stay on main if possible
    let _ = Command::new("git")
        .current_dir(&repo.path)
        .args(["checkout", "main"])
        .status();

    // Stage project files only (gitignore respected; never force secrets)
    let st = Command::new("git")
        .current_dir(&repo.path)
        .args(["add", "-A"])
        .status()?;
    if !st.success() {
        bail!("git add failed");
    }

    let msg = format!(
        "xbgst: {} milestone on {} (strict improvement, gates green).\n\nWave kind: {}. Auth: ChatGPT OAuth Titanium via sekhmet L3.\n",
        kind.as_str(),
        repo.name,
        kind.as_str()
    );
    let commit = Command::new("git")
        .current_dir(&repo.path)
        .args(["commit", "-m", &msg])
        .output()?;
    if !commit.status.success() {
        let err = String::from_utf8_lossy(&commit.stderr);
        if err.contains("nothing to commit") {
            return Ok(ShipRecord {
                repo: repo.name.clone(),
                approved: false,
                reason: "BLOCKED: nothing to commit after stage".into(),
                commit: None,
                pushed: false,
                push_err: None,
                dirty_before,
                dirty_after,
                gate_ok,
                gate_detail,
            });
        }
        bail!("git commit failed: {err}");
    }
    let sha = git_rev_parse(&repo.path)?;

    let (pushed, push_err) = if do_push {
        push_main(&repo.path)?
    } else {
        (false, Some("no_push flag set".into()))
    };

    if !pushed {
        if let Some(ref e) = push_err {
            eprintln!("xbgst-l3-orch: push failed for {}: {e}", repo.name);
        }
    }

    Ok(ShipRecord {
        repo: repo.name.clone(),
        approved: true,
        reason: format!(
            "APPROVED: strict improvement on {} ({})",
            repo.name,
            kind.as_str()
        ),
        commit: Some(sha),
        pushed,
        push_err,
        dirty_before,
        dirty_after,
        gate_ok,
        gate_detail,
    })
}

/// Remove non-product noise that sekhmet/Titanium leaves and that breaks "strict improvement".
fn sanitize_worktree(repo: &Path) {
    // Untracked noise only — never discard tracked edits.
    let patterns = [
        "**/__pycache__",
        "**/*.pyc",
        "**/target/tmp",
        "**/.DS_Store",
        "**/ndjson.ship.out",
        "**/swarm.ship.stderr.log",
    ];
    for pat in patterns {
        let _ = Command::new("git")
            .current_dir(repo)
            .args(["clean", "-fd", "-e", "!.env", "--", pat])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    // Explicit known junk dirs under benchmarks
    {
        let p = repo.join(
            "benchmarks/dry-hump/telemetry-e2e-model-questions/__pycache__",
        );
        if p.exists() {
            let _ = fs::remove_dir_all(&p);
        }
    }
}

/// Push `main` to `origin` with one retry. Returns (ok, err_snip).
fn push_main(repo: &Path) -> Result<(bool, Option<String>)> {
    for attempt in 1..=2 {
        let p = Command::new("git")
            .current_dir(repo)
            .args(["push", "-u", "origin", "main"])
            .output()?;
        if p.status.success() {
            return Ok((true, None));
        }
        let err = format!(
            "attempt {attempt}: {}",
            String::from_utf8_lossy(&p.stderr).trim()
        );
        if attempt == 1 {
            // fetch in case remote moved; still no force-push
            let _ = Command::new("git")
                .current_dir(repo)
                .args(["fetch", "origin", "main"])
                .status();
            continue;
        }
        return Ok((false, Some(err.chars().take(800).collect())));
    }
    Ok((false, Some("push exhausted retries".into())))
}

/// Gate ladder for Rust crates:
/// 1) `cargo check --all-targets` (required)
/// 2) `cargo test --lib` (fast unit tests; required if package has lib)
/// 3) full `cargo test --all-targets` with wall timeout (best-effort; fail only if check/lib ok but tests error)
///
/// Non-Rust: `.git` present counts as pass (docs/marketplace).
/// Override: `XBGST_SHIP_GATE=check|lib|full` (default `lib`).
fn run_gates(repo: &Path) -> Result<(bool, Option<String>)> {
    let mode = std::env::var("XBGST_SHIP_GATE").unwrap_or_else(|_| "lib".into());
    if !repo.join("Cargo.toml").is_file() {
        let ok = repo.join(".git").exists();
        return Ok((
            ok,
            Some(if ok {
                "non-rust git repo".into()
            } else {
                "not a git repo".into()
            }),
        ));
    }

    let check = cargo_status(repo, &["check", "--all-targets"], 300)?;
    if !check.0 {
        return Ok((
            false,
            Some(format!("cargo check failed: {}", snip(&check.1))),
        ));
    }
    if mode.eq_ignore_ascii_case("check") {
        return Ok((true, Some("cargo check ok".into())));
    }

    // Prefer --lib when present (faster, less flaky under concurrent orch).
    let has_lib = repo.join("src/lib.rs").is_file()
        || fs::read_to_string(repo.join("Cargo.toml"))
            .map(|t| t.contains("[lib]") || t.contains("path = \"src/lib.rs\""))
            .unwrap_or(false);

    if has_lib || mode.eq_ignore_ascii_case("lib") {
        let lib = cargo_status(
            repo,
            &["test", "--lib", "--", "--test-threads=2"],
            240,
        )?;
        if !lib.0 {
            return Ok((
                false,
                Some(format!("cargo test --lib failed: {}", snip(&lib.1))),
            ));
        }
        if !mode.eq_ignore_ascii_case("full") {
            return Ok((true, Some("cargo check + test --lib ok".into())));
        }
    }

    let full = cargo_status(
        repo,
        &["test", "--all-targets", "--", "--test-threads=2"],
        360,
    )?;
    if full.0 {
        Ok((true, Some("cargo check + full test ok".into())))
    } else {
        Ok((
            false,
            Some(format!("cargo test --all-targets failed: {}", snip(&full.1))),
        ))
    }
}

fn cargo_status(repo: &Path, args: &[&str], timeout_secs: u64) -> Result<(bool, String)> {
    // Prefer GNU timeout when present so hung tests cannot block the orch forever.
    let use_timeout = which::which("timeout").is_ok();
    let mut cmd = if use_timeout {
        let mut c = Command::new("timeout");
        c.arg(timeout_secs.to_string()).arg("cargo");
        for a in args {
            c.arg(a);
        }
        c
    } else {
        let mut c = Command::new("cargo");
        for a in args {
            c.arg(a);
        }
        c
    };
    cmd.current_dir(repo)
        .env("CARGO_TERM_COLOR", "never")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let out = cmd.output()?;
    let mut blob = String::new();
    blob.push_str(&String::from_utf8_lossy(&out.stdout));
    blob.push_str(&String::from_utf8_lossy(&out.stderr));
    // timeout(1) exit 124 = timed out
    let ok = out.status.success();
    if !ok && use_timeout && out.status.code() == Some(124) {
        blob.push_str("\n(timeout)");
    }
    Ok((ok, blob))
}

fn snip(s: &str) -> String {
    let t = s.trim();
    // keep last lines — usually the error
    let lines: Vec<&str> = t.lines().rev().take(12).collect();
    let out: Vec<&str> = lines.into_iter().rev().collect();
    if out.is_empty() {
        return "no output".into();
    }
    let joined = out.join(" | ");
    joined.chars().take(400).collect()
}

fn git_dirty(repo: &Path) -> Result<bool> {
    let o = Command::new("git")
        .current_dir(repo)
        .args(["status", "--porcelain"])
        .output()?;
    Ok(!String::from_utf8_lossy(&o.stdout).trim().is_empty())
}

fn git_has_diff_vs_head(repo: &Path) -> Result<bool> {
    let o = Command::new("git")
        .current_dir(repo)
        .args(["diff", "--stat", "HEAD"])
        .output()?;
    let staged = Command::new("git")
        .current_dir(repo)
        .args(["diff", "--cached", "--stat"])
        .output()?;
    let untracked = Command::new("git")
        .current_dir(repo)
        .args(["ls-files", "--others", "--exclude-standard"])
        .output()?;
    Ok(!String::from_utf8_lossy(&o.stdout).trim().is_empty()
        || !String::from_utf8_lossy(&staged.stdout).trim().is_empty()
        || !String::from_utf8_lossy(&untracked.stdout).trim().is_empty())
}

fn git_rev_parse(repo: &Path) -> Result<String> {
    let o = Command::new("git")
        .current_dir(repo)
        .args(["rev-parse", "--short", "HEAD"])
        .output()?;
    Ok(String::from_utf8_lossy(&o.stdout).trim().to_string())
}

/// Short godspeed **directive** only (4 rules + concurrent tools + Rust lock).
/// Never inject filter/velocity (trilogy stays judge-only).
const GODSPEED_DIRECTIVE: &str = "\
You are Godspeed-enabled. \
1. Name the axes. \
2. Iterate cheap, in parallel. \
3. Keep moves that improve any axis and harm none. \
4. Don't aim — let the frontier walk itself. \
IMMEDIATELY STOP ASKING CLARIFYING QUESTIONS. \
Execute tool calls concurrently in large batches. Do not serialize what can run in parallel. \
Do not output philosophical reasoning or verbose plans. Act directly via tool calls. \
Language lock: only Rust. No Python.";

fn pick_scope_repo(repos: &[LocalRepo], wave: u64, kind: WaveKind) -> Option<LocalRepo> {
    if repos.is_empty() || kind == WaveKind::Issues {
        return None;
    }
    Some(repos[((wave as usize).saturating_sub(1)) % repos.len()].clone())
}

fn build_wave_tasks(
    issues: &[Issue],
    repos: &[LocalRepo],
    kind: WaveKind,
    wave: u64,
    n: usize,
    scope: Option<&LocalRepo>,
) -> (Vec<String>, Vec<String>) {
    let mut lines = Vec::with_capacity(n);
    let mut roles = Vec::new();
    for i in 0..n {
        let role = ROLES[i % ROLES.len()];
        roles.push(role.to_string());
        let prompt = match kind {
            WaveKind::Issues => {
                if issues.is_empty() {
                    format!(
                        "{GODSPEED_DIRECTIVE} ROLE={role} | No open issues loaded. Propose how to discover work. Under 20 lines."
                    )
                } else {
                    let issue = &issues[(wave as usize + i) % issues.len()];
                    let body: String = issue.body.chars().take(500).collect();
                    format!(
                        "{GODSPEED_DIRECTIVE} You are xbgst `{role}` (Titanium OAuth). \
ISSUE {repo}#{num}: {title} | URL {url} | BODY {body} | \
Produce role-specific output (labrat=probe+gate, scout=prior-art, reviewer=risks, executor=Rust sketch, \
critic=attacks, connector=cross-links 2 issues, sentinel=security, distiller=5-bullet next session plan, \
simplifier=what to delete, mutation-tester=one test claim, scribe=commit message draft). \
Max 35 lines. Start with ROLE={role} ISSUE={repo}#{num}.",
                        role = role,
                        repo = issue.repo,
                        num = issue.number,
                        title = issue.title.replace('|', "/"),
                        url = issue.url,
                        body = body.replace('|', "/").replace('\n', " "),
                    )
                }
            }
            WaveKind::Review | WaveKind::Polish | WaveKind::Improve => {
                let repo_name = scope
                    .map(|r| r.name.as_str())
                    .or_else(|| repos.first().map(|r| r.name.as_str()))
                    .unwrap_or("workspace");
                let verb = kind.as_str();
                format!(
                    "{GODSPEED_DIRECTIVE} You are xbgst `{role}` working INSIDE scoped repo `{repo}` (Titanium may mutate workspace). \
Wave kind={verb}. Language lock: Rust only for code changes. \
1) Inspect the tree (src/, tests/, README, Cargo.toml). \
2) Make a STRICT IMPROVEMENT only: fix a real bug, add a missing test, tighten docs/AGENTS, remove dead code, or improve sekhmet/xbgst wiring. \
3) Do not drive-by reformat whole trees. Prefer small diffs. \
4) Run or outline `cargo test` / `cargo check` for this crate. \
5) If you change files, leave them saved; judge will commit+push main only if gates pass. \
6) End with: CHANGE_SUMMARY: <one line> | GATE: <cmd> | ROLE={role}. \
If nothing is a strict improvement, write NO_IMPROVEMENT and why.",
                    role = role,
                    repo = repo_name,
                    verb = verb,
                )
            }
        };
        // Always inject connector mandate on connector role
        let prompt = if role == "connector" {
            format!(
                "{prompt} | MANDATORY connector: name cross-axis pattern linking issues↔local polish work."
            )
        } else {
            prompt
        };
        lines.push(prompt.replace('\n', " "));
    }
    roles.truncate(12);
    (lines, roles)
}

#[derive(Default)]
struct NdStats {
    tasks: usize,
    ok: usize,
    fail: usize,
    timeout: usize,
    error: usize,
    usage_tokens_sum: u64,
    usage_tokens_n: usize,
}

fn summarize_ndjson(path: &Path) -> Result<NdStats> {
    let mut s = NdStats::default();
    if !path.is_file() {
        return Ok(s);
    }
    let f = fs::File::open(path)?;
    for line in std::io::BufReader::new(f).lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        s.tasks += 1;
        let v: serde_json::Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => {
                s.error += 1;
                continue;
            }
        };
        match v.get("status").and_then(|x| x.as_str()).unwrap_or("") {
            "ok" => s.ok += 1,
            "timeout" => s.timeout += 1,
            "error" => s.error += 1,
            _ => s.fail += 1,
        }
        if let Some(t) = v
            .get("usage_tokens")
            .and_then(|x| x.as_u64())
            .or_else(|| {
                v.pointer("/provenance/usage_tokens")
                    .and_then(|x| x.as_u64())
            })
        {
            s.usage_tokens_sum += t;
            s.usage_tokens_n += 1;
        }
    }
    Ok(s)
}

fn load_issues(path: &Path) -> Result<Vec<Issue>> {
    let raw = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let raw = raw.trim();
    let mut out = Vec::new();
    if raw.starts_with('[') {
        let items: Vec<GhIssue> = serde_json::from_str(raw)?;
        for g in items {
            out.push(normalize_issue(g));
        }
    } else {
        for (i, line) in raw.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let g: GhIssue = serde_json::from_str(line)
                .with_context(|| format!("JSONL line {}", i + 1))?;
            out.push(normalize_issue(g));
        }
    }
    out.sort_by(|a, b| (&a.repo, a.number).cmp(&(&b.repo, b.number)));
    out.dedup_by(|a, b| a.repo == b.repo && a.number == b.number);
    Ok(out)
}

fn normalize_issue(g: GhIssue) -> Issue {
    let repo = g
        .repository
        .as_ref()
        .and_then(|r| r.name_with_owner.clone().or_else(|| r.name.clone()))
        .unwrap_or_else(|| "unknown/unknown".into());
    let url = g
        .url
        .unwrap_or_else(|| format!("https://github.com/{repo}/issues/{}", g.number));
    Issue {
        repo,
        number: g.number,
        title: g.title,
        url,
        body: g.body.unwrap_or_default(),
    }
}

fn load_repos(path: Option<&Path>) -> Result<Vec<LocalRepo>> {
    let mut out = Vec::new();
    if let Some(p) = path {
        if p.is_file() {
            for line in fs::read_to_string(p)?.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                let pb = PathBuf::from(line);
                if pb.is_dir() && pb.join(".git").exists() {
                    let name = pb
                        .file_name()
                        .map(|s| s.to_string_lossy().into_owned())
                        .unwrap_or_else(|| line.to_string());
                    out.push(LocalRepo { path: pb, name });
                }
            }
        }
    }
    // Default high-value local trees if file empty.
    // Prefer sekhmetalt over xbrd-spark when both exist (same origin; avoid dual dirty).
    if out.is_empty() {
        for name in [
            "sekhmetalt",
            "xbrd-selector",
            "grok-marketplace",
            "xbrd-grok",
        ] {
            let pb = PathBuf::from(format!("/home/vgpnk1337/Projects/{name}"));
            if pb.is_dir() && pb.join(".git").exists() {
                out.push(LocalRepo {
                    path: pb,
                    name: name.into(),
                });
            }
        }
        // Only include xbrd-spark if sekhmetalt missing
        if !out.iter().any(|r| r.name == "sekhmetalt") {
            let pb = PathBuf::from("/home/vgpnk1337/Projects/xbrd-spark");
            if pb.is_dir() && pb.join(".git").exists() {
                out.push(LocalRepo {
                    path: pb,
                    name: "xbrd-spark".into(),
                });
            }
        }
    }
    // Dedupe by canonical path
    out.sort_by(|a, b| a.path.cmp(&b.path));
    out.dedup_by(|a, b| a.path == b.path);
    Ok(out)
}

/// Aggregated orch counters + pointers for `write_summary` (avoids clippy too_many_arguments).
struct OrchSummary<'a> {
    telem: &'a Path,
    waves: u64,
    wall_secs: f64,
    tasks: usize,
    ok: usize,
    fail: usize,
    tokens: u64,
    ships: usize,
    cli: &'a Cli,
    issues_n: usize,
    repos_n: usize,
}

fn write_summary(s: &OrchSummary<'_>) -> Result<()> {
    let OrchSummary {
        telem,
        waves,
        wall_secs,
        tasks,
        ok,
        fail,
        tokens,
        ships,
        cli,
        issues_n,
        repos_n,
    } = s;
    let v = json!({
        "updated_at": chrono_now(),
        "waves_completed": waves,
        "wall_secs": wall_secs,
        "tasks": tasks,
        "ok": ok,
        "fail": fail,
        "usage_tokens_sum": tokens,
        "ships_approved": ships,
        "issues_n": issues_n,
        "repos_n": repos_n,
        "model": cli.model,
        "service_tier": cli.service_tier,
        "jobs": cli.jobs,
        "duration_mins_budget": cli.duration_mins,
        "auth": "chatgpt-oauth",
        "xbgst_ship": "commit+push main on strict improvement only",
    });
    fs::write(telem.join("summary.json"), serde_json::to_string_pretty(&v)?)?;
    let notes = format!(
        "# xbgst L3 lunch orch — notes\n\n\
updated: {}\n\
waves: {waves}\n\
wall_secs: {wall_secs:.1}\n\
tasks: {tasks} ok: {ok} fail: {fail}\n\
tokens: {tokens}\n\
ships_approved: {ships}\n\
model: {} tier: {} jobs: {}\n\
auth: **ChatGPT OAuth** (not platform API key)\n\
issues: {issues_n} | local repos: {repos_n}\n\
ship rule: APPROVED only if tree changed AND gates green → commit → push main\n\
telemetry: telemetry/waves.ndjson, telemetry/ships.ndjson, telemetry/waves/\n\
",
        chrono_now(),
        cli.model,
        cli.service_tier,
        cli.jobs
    );
    fs::write(telem.join("NOTES.md"), notes)?;
    Ok(())
}

fn write_json(path: &Path, v: &impl Serialize) -> Result<()> {
    fs::write(path, serde_json::to_string_pretty(v)?)?;
    Ok(())
}

fn clean_tmp_globs() -> Result<()> {
    let tmp = std::env::temp_dir();
    let prefixes = [
        "sekhmet-",
        "xbrd-spark-",
        "sekhmet-orch-",
        "sekhmet-12x-",
        "sekhmet-luna",
    ];
    if let Ok(rd) = fs::read_dir(&tmp) {
        for ent in rd.flatten() {
            let s = ent.file_name().to_string_lossy().into_owned();
            if prefixes.iter().any(|p| s.starts_with(p)) {
                let p = ent.path();
                if p.is_dir() {
                    let _ = fs::remove_dir_all(&p);
                } else {
                    let _ = fs::remove_file(&p);
                }
            }
        }
    }
    Ok(())
}

fn which_sekhmet() -> Result<PathBuf> {
    if let Ok(p) = std::env::var("SEKHMET_BIN") {
        return Ok(PathBuf::from(p));
    }
    which::which("sekhmet")
        .or_else(|_| which::which("xbrd-spark"))
        .context("sekhmet not on PATH")
}

fn chrono_now() -> String {
    Command::new("date")
        .args(["-u", "+%Y-%m-%dT%H:%M:%SZ"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".into())
}

fn uuid_simple() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let t = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{t:x}")
}
