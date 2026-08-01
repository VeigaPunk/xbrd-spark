#!/usr/bin/env python3
"""Rebuild live Q→A for e2e model-authored tasks via gpt-5.4-mini."""
from __future__ import annotations

import json
import os
import re
import shutil
import subprocess
import sys
import textwrap
import time
from pathlib import Path

BASE = Path(__file__).resolve().parent
QA_DIR = BASE / "qa-ship"
JOBS = os.environ.get("QA_SHIP_JOBS", "12")
MODEL = os.environ.get("XBRD_SPARK_MODEL", "gpt-5.4-mini")


def run_key(p: Path) -> int:
    m = re.match(r"e2e(\d+)", p.name)
    return int(m.group(1)) if m else 999


def good_qa(path: Path) -> bool:
    if not path.is_file() or path.stat().st_size < 40_000:
        return False
    t = path.read_text(errors="ignore")
    return t.count("**status:** ok") >= 50 and t.count("_(empty)_") <= 10


def harvest(ndjson: Path) -> tuple[dict, int, int]:
    by_q: dict = {}
    ok = fail = 0
    if not ndjson.is_file():
        return by_q, ok, fail
    for line in ndjson.read_text().splitlines():
        if not line.strip():
            continue
        try:
            o = json.loads(line)
        except Exception:
            continue
        cmdl = (o.get("provenance") or {}).get("cmdline") or []
        q = cmdl[-1] if cmdl else None
        status = o.get("status")
        if status == "ok":
            ok += 1
        else:
            fail += 1
        ans = ""
        rp = o.get("result_path")
        if rp and Path(rp).is_file():
            try:
                r = json.loads(Path(rp).read_text())
                ans = r.get("stdout") or r.get("text") or ""
            except Exception as e:
                ans = f"(result read error: {e})"
        if q:
            by_q[q] = {
                "status": status,
                "spark_id": o.get("spark_id"),
                "duration_ms": (o.get("provenance") or {}).get("duration_ms"),
                "answer": ans,
                "usage_tokens": o.get("usage_tokens")
                or (o.get("provenance") or {}).get("usage_tokens"),
            }
    return by_q, ok, fail


def write_qa(run: Path, qs: list[str], by_q: dict, ok: int, fail: int, wall: float, ec: int) -> tuple[str, int]:
    lines = [
        f"# Q→A — `{run.name}`\n",
        f"answer_model: `{MODEL}` (Titanium; spark quota blocked until 2026-08-08)\n",
        f"questions={len(qs)} matched={len(by_q)} ok={ok} fail={fail} "
        f"swarm_wall_s={wall:.3f} swarm_ec={ec}\n",
    ]
    matched = 0
    for n, q in enumerate(qs, 1):
        rec = by_q.get(q)
        lines.append(f"## Q{n:02d}\n")
        lines.append(f"**Q:** {q}\n")
        if rec:
            matched += 1
            lines.append(
                f"**status:** {rec['status']} · **spark_id:** `{rec['spark_id']}` · "
                f"**duration_ms:** {rec['duration_ms']}\n"
            )
            if rec.get("usage_tokens") is not None:
                lines.append(f"**usage_tokens:** {rec['usage_tokens']}\n")
            a = (rec.get("answer") or "").strip() or "_(empty)_"
            lines.append(f"**A:**\n\n{a}\n")
        else:
            lines.append("**status:** MISSING\n\n**A:**\n\n_(no match)_\n")
        lines.append("---\n")
    body = "\n".join(lines)
    (QA_DIR / f"{run.name}-QA.md").write_text(body)
    return body, matched


def main() -> int:
    QA_DIR.mkdir(exist_ok=True)
    runs = sorted(
        [
            p
            for p in (BASE / "runs").iterdir()
            if p.is_dir()
            and p.name.startswith("e2e")
            and "e2e06b" not in p.name
            and "e2e06c" not in p.name
        ],
        key=run_key,
    )
    summary: list[str] = []
    t0 = time.time()
    print(f"model={MODEL} jobs={JOBS} runs={len(runs)}", flush=True)

    for i, run in enumerate(runs, 1):
        outp = QA_DIR / f"{run.name}-QA.md"
        tasks = run / "tasks.txt"
        qs = [
            ln.strip()
            for ln in tasks.read_text().splitlines()
            if ln.strip() and not ln.strip().startswith("#")
        ]
        if good_qa(outp):
            print(f"SKIP good {run.name} bytes={outp.stat().st_size}", flush=True)
            summary.append(f"{run.name}\tSKIP_GOOD")
            continue

        root = Path(f"/tmp/sekhmet-qa-ship-{run.name}-{os.getpid()}")
        shutil.rmtree(root, ignore_errors=True)
        root.mkdir(parents=True)
        ndjson = run / "ndjson.ship.out"
        err = run / "swarm.ship.stderr.log"
        print(f"===== {i}/{len(runs)} {run.name} qs={len(qs)} =====", flush=True)
        t1 = time.time()
        env = os.environ.copy()
        env["XBRD_SPARK_MODEL"] = MODEL
        cmd = [
            "sekhmet",
            "swarm",
            "-j",
            JOBS,
            "--direct",
            "--timeout",
            "180",
            "--root",
            str(root),
            "-f",
            str(tasks),
        ]
        with open(ndjson, "w") as out, open(err, "w") as er:
            proc = subprocess.run(cmd, stdout=out, stderr=er, env=env)
        wall = time.time() - t1
        by_q, ok, fail = harvest(ndjson)
        _, matched = write_qa(run, qs, by_q, ok, fail, wall, proc.returncode)
        row = (
            f"{run.name}\tqs={len(qs)}\tmatched={matched}\tok={ok}\tfail={fail}"
            f"\twall={wall:.1f}\tec={proc.returncode}"
        )
        summary.append(row)
        print(row, flush=True)
        shutil.rmtree(root, ignore_errors=True)

    # assemble ALL from disk (fresh)
    parts = []
    for run in runs:
        p = QA_DIR / f"{run.name}-QA.md"
        if p.is_file():
            parts.append(p.read_text())
    wall_all = time.time() - t0
    header = textwrap.dedent(
        f"""\
        # Sekhmet e2e — ALL Q→A (model-authored questions × live answers)

        Each OpenCode model generated its own question list; answers via Sekhmet `{MODEL}`.
        Original campaign used `gpt-5.3-codex-spark` but answer bodies were cleaned with `--no-keep`;
        spark model hit usage limit until 2026-08-08.

        rebuild_model: {MODEL}
        rebuild_jobs: {JOBS}
        rebuild_wall_seconds={wall_all:.3f}

        """
    )
    (QA_DIR / "ALL-QA.md").write_text(header + "\n\n".join(parts))
    (QA_DIR / "SUMMARY.tsv").write_text("\n".join(summary) + "\n")
    print("DONE wall", wall_all, "ALL bytes", (QA_DIR / "ALL-QA.md").stat().st_size, flush=True)
    for s in summary:
        print(s, flush=True)
    return 0


if __name__ == "__main__":
    sys.exit(main())
