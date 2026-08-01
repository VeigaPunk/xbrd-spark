//! Live Titanium smoke — ignored by default (network/auth/cost).
//! Run: `XBRD_SPARK_LIVE=1 cargo test --test live_smoke -- --ignored --nocapture`
use std::process::Command;
use tempfile::TempDir;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_sekhmet")
}

#[test]
#[ignore = "live Titanium; requires CODEX auth and network"]
fn live_direct_one_shot_emits_ndjson() {
    if std::env::var("XBRD_SPARK_LIVE").ok().as_deref() != Some("1") {
        eprintln!("skip: set XBRD_SPARK_LIVE=1");
        return;
    }
    let root = TempDir::new().unwrap();
    let out = Command::new(bin())
        .args([
            "run",
            "--direct",
            "--no-keep",
            "--timeout",
            "120",
            "--task",
            "Reply with exactly: pong",
            "--root",
        ])
        .arg(root.path())
        .output()
        .expect("spawn");
    let stdout = String::from_utf8_lossy(&out.stdout);
    eprintln!("status={:?}\n{}", out.status, stdout);
    assert!(
        stdout.contains("spark_id"),
        "expected NDJSON record on stdout"
    );
    if let Some(line) = stdout.lines().find(|l| l.contains("spark_id")) {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
            eprintln!("usage_tokens={:?}", v.get("usage_tokens"));
        }
    }
}
