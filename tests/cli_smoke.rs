//! CLI smoke: dry-run → collect → status → gc via the built binary.
use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_xbrd-spark")
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
        .args([
            "gc",
            "--max-age",
            "0",
            "--root",
            root.to_str().unwrap(),
        ])
        .output()
        .expect("gc");
    assert!(
        gc.status.success(),
        "gc: {}",
        String::from_utf8_lossy(&gc.stderr)
    );
}
