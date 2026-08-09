//! harvest-qa — build ALL-QA.md from sekhmet swarm ndjson + result.json + tasks file.
//!
//! Q: tasks line matched by HARD* prefix / substring in cmdline, else index order.
//! A: result.json stdout (or (EMPTY)).
//! Stamps: model_id, model_reasoning_effort, service_tier from cmdline or CLI overrides.

use anyhow::{bail, Context, Result};
use clap::Parser;
use serde::Deserialize;
use serde_json::Value;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[command(name = "harvest-qa", about = "Harvest sekhmet results into ALL-QA.md")]
struct Args {
    /// Run directory containing ndjson.out, results/, optional summary.json
    #[arg(long)]
    run_dir: Option<PathBuf>,

    /// Live sekhmet root (sparks with in/task.md + out/result.json)
    #[arg(long)]
    root: Option<PathBuf>,

    /// Tasks file: one task per non-empty line
    #[arg(long)]
    tasks: PathBuf,

    /// Output ALL-QA.md path
    #[arg(long)]
    out: PathBuf,

    #[arg(long)]
    model_id: Option<String>,

    #[arg(long)]
    effort: Option<String>,

    #[arg(long)]
    tier: Option<String>,

    #[arg(long)]
    pack: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ResultJson {
    status: Option<String>,
    stdout: Option<String>,
    #[allow(dead_code)]
    stderr: Option<String>,
    #[allow(dead_code)]
    exit: Option<i32>,
    duration_ms: Option<u64>,
    usage_tokens: Option<u64>,
}

#[derive(Debug)]
struct SparkRow {
    spark_id: String,
    status: String,
    result_path: Option<PathBuf>,
    usage_tokens: Option<u64>,
    duration_ms: Option<u64>,
    model: Option<String>,
    cmdline: Vec<String>,
    task_from_cmdline: Option<String>,
    task_from_file: Option<String>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    if args.run_dir.is_none() && args.root.is_none() {
        bail!("need --run-dir and/or --root");
    }

    let tasks = load_tasks(&args.tasks)?;
    let mut rows = Vec::new();

    if let Some(ref run_dir) = args.run_dir {
        let ndjson = run_dir.join("ndjson.out");
        if ndjson.is_file() {
            rows = parse_ndjson(&ndjson)?;
        }
        // Resolve result paths into run_dir/results/<id>.json when live path gone
        for row in &mut rows {
            let local = run_dir.join("results").join(format!("{}.json", row.spark_id));
            if local.is_file() {
                row.result_path = Some(local);
            } else if let Some(ref rp) = row.result_path {
                if !rp.is_file() {
                    row.result_path = None;
                }
            }
        }
    }

    if rows.is_empty() {
        if let Some(ref root) = args.root {
            rows = scan_root(root)?;
        }
    } else if let Some(ref root) = args.root {
        // Fill task.md when missing
        for row in &mut rows {
            if row.task_from_file.is_none() {
                let p = root.join(&row.spark_id).join("in").join("task.md");
                if p.is_file() {
                    row.task_from_file = Some(fs::read_to_string(&p)?.trim().to_string());
                }
            }
            if row.result_path.as_ref().map(|p| !p.is_file()).unwrap_or(true) {
                let p = root.join(&row.spark_id).join("out").join("result.json");
                if p.is_file() {
                    row.result_path = Some(p);
                }
            }
        }
    }

    if rows.is_empty() {
        // Fallback: only results/*.json under run_dir (no ndjson) — dry structure
        if let Some(ref run_dir) = args.run_dir {
            let res_dir = run_dir.join("results");
            if res_dir.is_dir() {
                let mut ids: Vec<_> = fs::read_dir(&res_dir)?
                    .filter_map(|e| e.ok())
                    .filter(|e| {
                        e.path()
                            .extension()
                            .map(|x| x == "json")
                            .unwrap_or(false)
                    })
                    .map(|e| e.path())
                    .collect();
                ids.sort();
                for p in ids {
                    let sid = p
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("unknown")
                        .to_string();
                    rows.push(SparkRow {
                        spark_id: sid,
                        status: "unknown".into(),
                        result_path: Some(p),
                        usage_tokens: None,
                        duration_ms: None,
                        model: None,
                        cmdline: vec![],
                        task_from_cmdline: None,
                        task_from_file: None,
                    });
                }
            }
        }
    }

    // Identity stamps
    let (cmd_model, cmd_effort, cmd_tier) = stamps_from_rows(&rows);
    let summary_stamps = args
        .run_dir
        .as_ref()
        .map(|d| read_summary_stamps(d))
        .transpose()?
        .flatten();

    let model_id = args
        .model_id
        .or(cmd_model)
        .or_else(|| summary_stamps.as_ref().and_then(|s| s.0.clone()))
        .unwrap_or_else(|| "unknown".into());
    let effort = args
        .effort
        .or(cmd_effort)
        .or_else(|| summary_stamps.as_ref().and_then(|s| s.1.clone()))
        .unwrap_or_else(|| "low".into());
    // never stamp host medium as truth for sekhmet inject path
    let effort = if effort == "medium" || effort == "high" {
        "low".into()
    } else {
        effort
    };
    let tier = args
        .tier
        .or(cmd_tier)
        .or_else(|| summary_stamps.as_ref().and_then(|s| s.2.clone()))
        .unwrap_or_else(|| "fast".into());

    let run_id = args
        .run_dir
        .as_ref()
        .and_then(|d| d.file_name())
        .and_then(|s| s.to_str())
        .unwrap_or("run")
        .to_string();

    let pack = args.pack.clone().unwrap_or_else(|| {
        if run_id.starts_with("hard10") {
            "hard10".into()
        } else if run_id.starts_with("ethics") {
            "ethics".into()
        } else {
            "unknown".into()
        }
    });

    let (jobs, timeout, wall, ok, fail, to) = summary_counts(args.run_dir.as_deref());

    // Match tasks to rows
    let mut used = vec![false; tasks.len()];
    let mut matched: Vec<(String, SparkRow, ResultJson)> = Vec::new();

    for row in rows {
        let q_hint = row
            .task_from_file
            .clone()
            .or_else(|| row.task_from_cmdline.clone());
        let task_label = match_task(&tasks, &mut used, q_hint.as_deref());
        let rj = load_result(row.result_path.as_deref())?;
        matched.push((task_label, row, rj));
    }

    // Sort by task order (HARD01.. or index prefix)
    matched.sort_by(|a, b| task_sort_key(&a.0).cmp(&task_sort_key(&b.0)));

    if let Some(parent) = args.out.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut f = fs::File::create(&args.out)
        .with_context(|| format!("create {}", args.out.display()))?;

    writeln!(f, "# ALL-QA — {run_id}")?;
    writeln!(f, "- model_id: {model_id}")?;
    writeln!(f, "- model_reasoning_effort: {effort}")?;
    writeln!(f, "- service_tier: {tier}")?;
    writeln!(f, "- pack: {pack}")?;
    writeln!(
        f,
        "- jobs / timeout / wall: {} / {} / {}",
        jobs.unwrap_or_else(|| "?".into()),
        timeout.unwrap_or_else(|| "?".into()),
        wall.unwrap_or_else(|| "?".into())
    )?;
    writeln!(
        f,
        "- sparks_ok / fail / timeout: {} / {} / {}",
        ok.unwrap_or_else(|| count_status(&matched, "ok").to_string()),
        fail.unwrap_or_else(|| count_status(&matched, "fail").to_string()),
        to.unwrap_or_else(|| count_status(&matched, "timeout").to_string())
    )?;
    writeln!(f)?;

    for (task_label, row, rj) in &matched {
        let status = if !row.status.is_empty() && row.status != "unknown" {
            row.status.clone()
        } else {
            rj.status.clone().unwrap_or_else(|| "unknown".into())
        };
        let q = resolve_q(task_label, row, &tasks);
        let a = rj
            .stdout
            .as_ref()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "(EMPTY)".into());
        let tokens = row
            .usage_tokens
            .or(rj.usage_tokens)
            .map(|t| t.to_string())
            .unwrap_or_else(|| "null".into());
        let dur = row
            .duration_ms
            .or(rj.duration_ms)
            .map(|d| d.to_string())
            .unwrap_or_else(|| "null".into());
        let cmdline_stamp = format_cmdline_stamp(&row.cmdline, &model_id, &effort, &tier);

        writeln!(f, "## {task_label} — status={status}")?;
        writeln!(f, "### Q")?;
        writeln!(f, "{q}")?;
        writeln!(f, "### A")?;
        writeln!(f, "{a}")?;
        writeln!(f, "### meta")?;
        writeln!(f, "- spark_id: {}", row.spark_id)?;
        writeln!(f, "- usage_tokens: {tokens}")?;
        writeln!(f, "- duration_ms: {dur}")?;
        writeln!(f, "- cmdline_stamp: {cmdline_stamp}")?;
        writeln!(f)?;
    }

    eprintln!(
        "harvest-qa: wrote {} sections -> {}",
        matched.len(),
        args.out.display()
    );
    Ok(())
}

fn load_tasks(path: &Path) -> Result<Vec<String>> {
    let raw = fs::read_to_string(path).with_context(|| format!("read tasks {}", path.display()))?;
    Ok(raw
        .lines()
        .map(|l| l.trim_end().to_string())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect())
}

fn parse_ndjson(path: &Path) -> Result<Vec<SparkRow>> {
    let f = fs::File::open(path).with_context(|| format!("open {}", path.display()))?;
    let reader = BufReader::new(f);
    let mut rows = Vec::new();
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let v: Value = serde_json::from_str(&line)
            .with_context(|| format!("ndjson parse: {}", &line[..line.len().min(80)]))?;
        let spark_id = v
            .get("spark_id")
            .and_then(|x| x.as_str())
            .unwrap_or("unknown")
            .to_string();
        let status = v
            .get("status")
            .and_then(|x| x.as_str())
            .unwrap_or("unknown")
            .to_string();
        let result_path = v
            .get("result_path")
            .and_then(|x| x.as_str())
            .map(PathBuf::from);
        let usage_tokens = v.get("usage_tokens").and_then(|x| x.as_u64());
        let prov = v.get("provenance");
        let duration_ms = prov
            .and_then(|p| p.get("duration_ms"))
            .and_then(|x| x.as_u64());
        let model = prov
            .and_then(|p| p.get("model"))
            .and_then(|x| x.as_str())
            .map(|s| s.to_string());
        let cmdline: Vec<String> = prov
            .and_then(|p| p.get("cmdline"))
            .and_then(|c| c.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|x| x.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();
        let task_from_cmdline = extract_task_from_cmdline(&cmdline);
        rows.push(SparkRow {
            spark_id,
            status,
            result_path,
            usage_tokens,
            duration_ms,
            model,
            cmdline,
            task_from_cmdline,
            task_from_file: None,
        });
    }
    Ok(rows)
}

fn scan_root(root: &Path) -> Result<Vec<SparkRow>> {
    let mut rows = Vec::new();
    if !root.is_dir() {
        return Ok(rows);
    }
    let mut entries: Vec<_> = fs::read_dir(root)?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .map(|e| e.path())
        .collect();
    entries.sort();
    for dir in entries {
        let name = dir
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();
        if !name.starts_with("sp-") {
            continue;
        }
        let task_path = dir.join("in").join("task.md");
        let result_path = dir.join("out").join("result.json");
        let task_from_file = if task_path.is_file() {
            Some(fs::read_to_string(&task_path)?.trim().to_string())
        } else {
            None
        };
        let mut usage_tokens = None;
        let mut duration_ms = None;
        let mut status = "unknown".to_string();
        if result_path.is_file() {
            if let Ok(rj) = load_result(Some(&result_path)) {
                usage_tokens = rj.usage_tokens;
                duration_ms = rj.duration_ms;
                if let Some(s) = rj.status {
                    status = s;
                }
            }
        }
        rows.push(SparkRow {
            spark_id: name,
            status,
            result_path: if result_path.is_file() {
                Some(result_path)
            } else {
                None
            },
            usage_tokens,
            duration_ms,
            model: None,
            cmdline: vec![],
            task_from_cmdline: None,
            task_from_file,
        });
    }
    Ok(rows)
}

fn extract_task_from_cmdline(cmdline: &[String]) -> Option<String> {
    // Last arg is usually the full prompt; task after "---\n\n" or HARD* marker
    let prompt = cmdline.last()?;
    if let Some(idx) = prompt.find("\n---\n\n") {
        let rest = prompt[idx + 6..].trim();
        // strip trailing " | godspeed" if present
        let rest = rest.strip_suffix(" | godspeed").unwrap_or(rest).trim();
        if !rest.is_empty() {
            return Some(rest.to_string());
        }
    }
    // Find HARD or ETHICS style id
    for line in prompt.lines() {
        let t = line.trim();
        if t.starts_with("HARD") || t.starts_with("ETHICS") || (t.starts_with('E') && t.contains('_'))
        {
            let t = t.strip_suffix(" | godspeed").unwrap_or(t).trim();
            return Some(t.to_string());
        }
    }
    None
}

fn match_task(tasks: &[String], used: &mut [bool], hint: Option<&str>) -> String {
    if let Some(h) = hint {
        let htrim = h.trim();
        // exact
        for (i, t) in tasks.iter().enumerate() {
            if used[i] {
                continue;
            }
            if t == htrim || t.trim() == htrim {
                used[i] = true;
                return task_id_label(i, t);
            }
        }
        // prefix id HARD01_
        let prefix = htrim.split(':').next().unwrap_or(htrim);
        let prefix = prefix.split_whitespace().next().unwrap_or(prefix);
        for (i, t) in tasks.iter().enumerate() {
            if used[i] {
                continue;
            }
            if t.starts_with(prefix) || htrim.starts_with(t.chars().take(40).collect::<String>().as_str()) {
                used[i] = true;
                return task_id_label(i, t);
            }
            let tid = t.split(':').next().unwrap_or(t).split_whitespace().next().unwrap_or(t);
            if htrim.starts_with(tid) || t.starts_with(prefix) {
                used[i] = true;
                return task_id_label(i, t);
            }
        }
        // contains first 64 chars
        let key = htrim.chars().take(64).collect::<String>();
        for (i, t) in tasks.iter().enumerate() {
            if used[i] {
                continue;
            }
            if t.contains(&key) || htrim.contains(&t.chars().take(64).collect::<String>()) {
                used[i] = true;
                return task_id_label(i, t);
            }
        }
    }
    // first unused
    for (i, t) in tasks.iter().enumerate() {
        if !used[i] {
            used[i] = true;
            return task_id_label(i, t);
        }
    }
    format!("{:03}", used.len())
}

fn task_id_label(idx: usize, task: &str) -> String {
    let head = task.split(':').next().unwrap_or(task);
    let head = head.split_whitespace().next().unwrap_or(head);
    if head.starts_with("HARD") || head.starts_with("ETHICS") || head.contains('_') {
        head.to_string()
    } else {
        format!("{:03}", idx + 1)
    }
}

fn task_sort_key(label: &str) -> (u8, String) {
    // HARD01 before HARD10; numeric index
    if let Some(rest) = label.strip_prefix("HARD") {
        let num: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        if let Ok(n) = num.parse::<u32>() {
            return (0, format!("{n:04}"));
        }
    }
    if label.chars().all(|c| c.is_ascii_digit()) {
        return (1, format!("{:04}", label.parse::<u32>().unwrap_or(0)));
    }
    (2, label.to_string())
}

fn resolve_q(label: &str, row: &SparkRow, tasks: &[String]) -> String {
    if let Some(ref t) = row.task_from_file {
        if !t.is_empty() {
            return t.clone();
        }
    }
    if let Some(ref t) = row.task_from_cmdline {
        if !t.is_empty() {
            return t.clone();
        }
    }
    for t in tasks {
        let tid = t.split(':').next().unwrap_or(t).split_whitespace().next().unwrap_or(t);
        if tid == label || t.starts_with(label) {
            return t.clone();
        }
    }
    // index label
    if let Ok(n) = label.parse::<usize>() {
        if n >= 1 && n <= tasks.len() {
            return tasks[n - 1].clone();
        }
    }
    format!("(missing task for {label})")
}

fn load_result(path: Option<&Path>) -> Result<ResultJson> {
    match path {
        Some(p) if p.is_file() => {
            let s = fs::read_to_string(p)?;
            Ok(serde_json::from_str(&s).unwrap_or(ResultJson {
                status: Some("parse_error".into()),
                stdout: Some(s),
                stderr: None,
                exit: None,
                duration_ms: None,
                usage_tokens: None,
            }))
        }
        _ => Ok(ResultJson {
            status: Some("missing".into()),
            stdout: None,
            stderr: None,
            exit: None,
            duration_ms: None,
            usage_tokens: None,
        }),
    }
}

fn stamps_from_rows(rows: &[SparkRow]) -> (Option<String>, Option<String>, Option<String>) {
    for row in rows {
        let mut model = row.model.clone();
        let mut effort = None;
        let mut tier = None;
        let mut i = 0;
        while i < row.cmdline.len() {
            if row.cmdline[i] == "-m" && i + 1 < row.cmdline.len() {
                model = Some(row.cmdline[i + 1].clone());
            }
            if let Some(v) = row.cmdline[i].strip_prefix("model_reasoning_effort=") {
                effort = Some(v.to_string());
            }
            if let Some(v) = row.cmdline[i].strip_prefix("service_tier=") {
                tier = Some(v.to_string());
            }
            // -c model_reasoning_effort=low form
            if row.cmdline[i] == "-c" && i + 1 < row.cmdline.len() {
                let c = &row.cmdline[i + 1];
                if let Some(v) = c.strip_prefix("model_reasoning_effort=") {
                    effort = Some(v.to_string());
                }
                if let Some(v) = c.strip_prefix("service_tier=") {
                    tier = Some(v.to_string());
                }
            }
            i += 1;
        }
        if model.is_some() || effort.is_some() || tier.is_some() {
            return (model, effort, tier);
        }
    }
    (None, None, None)
}

fn read_summary_stamps(run_dir: &Path) -> Result<Option<(Option<String>, Option<String>, Option<String>)>> {
    let p = run_dir.join("summary.json");
    if !p.is_file() {
        return Ok(None);
    }
    let v: Value = serde_json::from_str(&fs::read_to_string(&p)?)?;
    Ok(Some((
        v.get("model_id")
            .or_else(|| v.get("model"))
            .and_then(|x| x.as_str())
            .map(|s| s.to_string()),
        v.get("model_reasoning_effort")
            .and_then(|x| x.as_str())
            .map(|s| s.to_string()),
        v.get("service_tier")
            .and_then(|x| x.as_str())
            .map(|s| s.to_string()),
    )))
}

fn summary_counts(
    run_dir: Option<&Path>,
) -> (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
) {
    let Some(d) = run_dir else {
        return (None, None, None, None, None, None);
    };
    let p = d.join("summary.json");
    if !p.is_file() {
        return (None, None, None, None, None, None);
    }
    let Ok(s) = fs::read_to_string(&p) else {
        return (None, None, None, None, None, None);
    };
    let Ok(v) = serde_json::from_str::<Value>(&s) else {
        return (None, None, None, None, None, None);
    };
    let jobs = v.get("jobs").map(|x| x.to_string());
    let timeout = v.get("timeout").map(|x| x.to_string());
    let wall = v
        .get("wall_seconds")
        .map(|x| x.to_string());
    let ok = v.get("sparks_ok").map(|x| x.to_string());
    let fail = v.get("sparks_fail").map(|x| x.to_string());
    let to = v.get("sparks_timeout").map(|x| x.to_string());
    (jobs, timeout, wall, ok, fail, to)
}

fn format_cmdline_stamp(cmdline: &[String], model: &str, effort: &str, tier: &str) -> String {
    if cmdline.is_empty() {
        return format!("-m {model} -c model_reasoning_effort={effort} -c service_tier={tier}");
    }
    let mut parts = Vec::new();
    let mut i = 0;
    while i < cmdline.len() {
        if cmdline[i] == "-m" && i + 1 < cmdline.len() {
            parts.push(format!("-m {}", cmdline[i + 1]));
            i += 2;
            continue;
        }
        if cmdline[i] == "-c" && i + 1 < cmdline.len() {
            let c = &cmdline[i + 1];
            if c.starts_with("model_reasoning_effort=") || c.starts_with("service_tier=") {
                parts.push(format!("-c {c}"));
            }
            i += 2;
            continue;
        }
        i += 1;
    }
    if parts.is_empty() {
        format!("-m {model} -c model_reasoning_effort={effort} -c service_tier={tier}")
    } else {
        parts.join(" ")
    }
}

fn count_status(matched: &[(String, SparkRow, ResultJson)], want: &str) -> usize {
    matched
        .iter()
        .filter(|(_, r, rj)| {
            let s = if r.status != "unknown" {
                r.status.as_str()
            } else {
                rj.status.as_deref().unwrap_or("")
            };
            s == want
        })
        .count()
}
