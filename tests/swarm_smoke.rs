//! Integration: sekhmet swarm dry-run pool + optional scope snapshot.
use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_sekhmet")
}

#[test]
fn swarm_dry_run_four_tasks_exit_zero() {
    let root = TempDir::new().unwrap();
    let tasks = root.path().join("tasks.txt");
    fs::write(&tasks, "alpha\nbeta\ngamma\ndelta\n").unwrap();

    let out = Command::new(bin())
        .args([
            "swarm",
            "--dry-run",
            "--no-keep",
            "-j",
            "4",
            "--tasks-file",
        ])
        .arg(&tasks)
        .arg("--root")
        .arg(root.path())
        .output()
        .expect("spawn sekhmet swarm");

    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    let lines = stdout
        .lines()
        .filter(|l| l.trim().starts_with('{'))
        .count();
    assert_eq!(lines, 4, "expected 4 NDJSON records, got:\n{stdout}");
}

#[test]
fn swarm_dry_run_with_scope_dir() {
    let root = TempDir::new().unwrap();
    let scope = TempDir::new().unwrap();
    fs::write(scope.path().join("marker.txt"), "scope-ok").unwrap();
    let tasks = root.path().join("tasks.txt");
    fs::write(&tasks, "task-with-scope\n").unwrap();

    let out = Command::new(bin())
        .args(["swarm", "--dry-run", "--no-keep", "-j", "1", "--tasks-file"])
        .arg(&tasks)
        .arg("--scope")
        .arg(scope.path())
        .arg("--root")
        .arg(root.path())
        .output()
        .expect("spawn sekhmet swarm scoped");

    assert!(
        out.status.success(),
        "stderr={}",
        String::from_utf8_lossy(&out.stderr)
    );
}
