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
    assert_eq!(records, 3, "expected 3 NDJSON spark records, stdout:\n{stdout}");
    let n = fs::read_dir(&root)
        .unwrap()
        .filter(|e| e.as_ref().map(|e| e.path().is_dir()).unwrap_or(false))
        .count();
    assert_eq!(n, 3);
}
