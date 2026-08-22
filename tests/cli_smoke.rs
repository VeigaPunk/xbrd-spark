//! CLI smoke: dry-run → collect → status → gc via the built binary.
//! Also covers sekhmet alias + swarm dry-run on Titanium-ready surface.
use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_xbrd-spark")
}

fn sekhmet_bin() -> &'static str {
    env!("CARGO_BIN_EXE_sekhmet")
}

#[test]
fn dry_run_collect_status_gc_exit_zero() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path();

    let out = Command::new(bin())
        .args([
            "run",
            "--dry-run",
            "--task",
            "cli-smoke",
            "--root",
            root.to_str().unwrap(),
            "--deterministic",
        ])
        .output()
        .expect("spawn xbrd-spark run");
    assert!(
        out.status.success(),
        "run failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let line = stdout.lines().last().expect("ndjson line");
    let v: serde_json::Value = serde_json::from_str(line).expect("ndjson parse");
    let id = v["spark_id"].as_str().expect("spark_id");
    assert!(id.starts_with("sp-"));

    let col = Command::new(bin())
        .args(["collect", id, "--root", root.to_str().unwrap()])
        .output()
        .expect("collect");
    assert!(
        col.status.success(),
        "collect: {}",
        String::from_utf8_lossy(&col.stderr)
    );

    let st = Command::new(bin())
        .args(["status", id, "--root", root.to_str().unwrap()])
        .output()
        .expect("status");
    assert!(
        st.status.success(),
        "status: {}",
        String::from_utf8_lossy(&st.stderr)
    );
    assert!(fs::metadata(root.join(id).join("meta.json")).is_ok());

    let gc = Command::new(bin())
        .args(["gc", "--max-age", "0", "--root", root.to_str().unwrap()])
        .output()
        .expect("gc");
    assert!(
        gc.status.success(),
        "gc: {}",
        String::from_utf8_lossy(&gc.stderr)
    );
}

#[test]
fn sekhmet_alias_swarm_dry_run() {
    let tmp = TempDir::new().unwrap();
    let root = tmp.path().join("root");
    fs::create_dir_all(&root).unwrap();
    let tasks = tmp.path().join("tasks.txt");
    fs::write(&tasks, "alpha\nbeta\ngamma\n").unwrap();

    let out = Command::new(sekhmet_bin())
        .args([
            "swarm",
            "--dry-run",
            "--jobs",
            "3",
            "--tasks-file",
            tasks.to_str().unwrap(),
            "--root",
            root.to_str().unwrap(),
        ])
        .output()
        .expect("spawn sekhmet swarm");
    assert!(
        out.status.success(),
        "swarm failed: {}\n{}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let mut records = 0usize;
    for line in stdout.lines() {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        if v.get("spark_id").is_some() {
            records += 1;
        }
    }
    assert_eq!(
        records, 3,
        "expected 3 NDJSON spark records, stdout:\n{stdout}"
    );
    let n = fs::read_dir(&root)
        .unwrap()
        .filter(|e| e.as_ref().map(|e| e.path().is_dir()).unwrap_or(false))
        .count();
    assert_eq!(n, 3);
}

#[cfg(unix)]
#[test]
fn sekhmet_actual_dispatch_uses_canonical_directive_and_one_closer() {
    use std::os::unix::fs::PermissionsExt;

    let tmp = TempDir::new().unwrap();
    let root = tmp.path().join("root");
    let fake_codex = tmp.path().join("fake-codex");
    fs::write(
        &fake_codex,
        b"#!/bin/sh\nlast=\nfor arg in \"$@\"; do last=$arg; done\nbase=${0%/*}\nprintf '%s' \"$last\" > \"$base/captured-prompt\"\n",
    )
    .unwrap();
    fs::set_permissions(&fake_codex, fs::Permissions::from_mode(0o755)).unwrap();

    let out = Command::new(sekhmet_bin())
        .env("CODEX_BIN", &fake_codex)
        .env_remove("XBRD_SPARK_MODEL")
        .env_remove("XBRD_SPARK_FALLBACK_MODEL")
        .env_remove("XBRD_SPARK_USE_FALLBACK")
        .env_remove("XBRD_SPARK_SERVICE_TIER")
        .args([
            "run",
            "--id",
            "sp-canonical-argv",
            "--task",
            "inspect this | GODSPEED | godspeed",
            "--root",
            root.to_str().unwrap(),
        ])
        .output()
        .expect("spawn sekhmet with fake codex");
    assert!(
        out.status.success(),
        "dispatch failed: stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    let prompt = fs::read_to_string(tmp.path().join("captured-prompt")).unwrap();
    assert!(
        prompt
            .as_bytes()
            .starts_with(xbrd_spark::GODSPEED_DIRECTIVE.as_bytes()),
        "actual argv omitted canonical directive"
    );
    assert!(prompt.ends_with(xbrd_spark::GODSPEED_PROMPT_SUFFIX));
    assert_eq!(
        prompt.matches(xbrd_spark::GODSPEED_PROMPT_SUFFIX).count(),
        1,
        "actual argv must end in one canonical closer: {prompt:?}"
    );

    let standalone = fs::read(root.join("sp-canonical-argv/in/godspeed.md")).unwrap();
    assert_eq!(standalone, xbrd_spark::GODSPEED_DIRECTIVE.as_bytes());

    let meta: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("sp-canonical-argv/meta.json")).unwrap())
            .unwrap();
    let cmdline = meta["cmdline"].as_array().unwrap();
    assert!(
        cmdline
            .iter()
            .any(|arg| arg.as_str() == Some("service_tier=default")),
        "neutral tier must be explicit in actual dispatch argv: {cmdline:?}"
    );
}

#[cfg(unix)]
#[test]
fn sekhmet_rejects_unsupported_tier_before_spawn() {
    use std::os::unix::fs::PermissionsExt;

    let tmp = TempDir::new().unwrap();
    let root = tmp.path().join("root");
    let fake_codex = tmp.path().join("fake-codex");
    fs::write(
        &fake_codex,
        b"#!/bin/sh\nbase=${0%/*}\ntouch \"$base/unexpected-spawn\"\n",
    )
    .unwrap();
    fs::set_permissions(&fake_codex, fs::Permissions::from_mode(0o755)).unwrap();

    let out = Command::new(sekhmet_bin())
        .env("CODEX_BIN", &fake_codex)
        .env("XBRD_SPARK_SERVICE_TIER", "flex")
        .env_remove("XBRD_SPARK_FALLBACK_MODEL")
        .env_remove("XBRD_SPARK_USE_FALLBACK")
        .args([
            "run",
            "--id",
            "sp-invalid-tier",
            "--task",
            "must not spawn",
            "--root",
            root.to_str().unwrap(),
        ])
        .output()
        .expect("spawn sekhmet invalid-tier probe");

    assert!(!out.status.success(), "unsupported tier must fail");
    let result: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("sp-invalid-tier/out/result.json")).unwrap())
            .unwrap();
    assert!(
        result["stderr"]
            .as_str()
            .unwrap_or_default()
            .contains("unsupported service tier 'flex'"),
        "unexpected result: {result:?}"
    );
    assert!(
        !tmp.path().join("unexpected-spawn").exists(),
        "dispatcher ran despite invalid tier"
    );
}
