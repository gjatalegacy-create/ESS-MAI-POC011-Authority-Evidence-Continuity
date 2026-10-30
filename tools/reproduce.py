#!/usr/bin/env python3
"""Run the bounded capsule checks. Creates a NEW evidence directory; never edits sources."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[1]

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("output", type=Path)
    parser.add_argument("--target-dir", type=Path)
    args = parser.parse_args()
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    env = os.environ.copy()
    if args.target_dir:
        env["CARGO_TARGET_DIR"] = str(args.target_dir.resolve())
    target = Path(env.get("CARGO_TARGET_DIR", str(ROOT / "target"))).resolve()
    cargo = shutil.which("cargo", path=env.get("PATH"))
    if not cargo:
        raise RuntimeError("cargo unavailable on PATH")
    commands = []
    def run(name, argv, timeout=300):
        start = time.monotonic()
        result = subprocess.run(argv, cwd=ROOT, env=env, capture_output=True, timeout=timeout)
        (output / (name + ".stdout.log")).write_bytes(result.stdout)
        (output / (name + ".stderr.log")).write_bytes(result.stderr)
        commands.append({"name": name, "argv": [str(x) for x in argv],
                         "exit_code": result.returncode, "wall_seconds": time.monotonic() - start})
        (output / "commands.json").write_text(json.dumps(commands, indent=2)+"\n", encoding="utf-8")
        print(f"{name}: exit={result.returncode}", flush=True)
        if result.returncode:
            raise RuntimeError(f"{name} failed; read {output}")
        return result
    versions = run("toolchain", [cargo, "--version"])
    (output / "environment.json").write_text(json.dumps({"os":platform.platform(),"machine":platform.machine(),
        "python":sys.version,"cargo":versions.stdout.decode().strip(),"cwd":str(ROOT),"target":str(target)},indent=2)+"\n",encoding="utf-8")
    run("format", [cargo, "fmt", "--all", "--", "--check"])
    run("test-debug", [cargo, "test", "--locked", "--offline"])
    run("test-release", [cargo, "test", "--locked", "--offline", "--release"])
    run("build-release", [cargo, "build", "--locked", "--offline", "--release"])
    binary = target / "release" / ("poc011.exe" if os.name == "nt" else "poc011")
    run("five-cycles", [str(binary), "demo", str(output / "demo")])
    run("inspect", [str(binary), "inspect", str(output / "demo" / "memory.wal")])
    run("independent-verifier", [sys.executable, str(ROOT / "tools" / "verify.py"), str(output / "demo"), "--self-test"])
    result = {"status":"passed", "binary_sha256":hashlib.sha256(binary.read_bytes()).hexdigest(),
              "lock_sha256":hashlib.sha256((ROOT/"Cargo.lock").read_bytes()).hexdigest(),
              "journal_sha256":hashlib.sha256((output/"demo"/"memory.wal").read_bytes()).hexdigest()}
    (output/"result.json").write_text(json.dumps(result,indent=2)+"\n",encoding="utf-8")
    print(json.dumps(result), flush=True)

if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        print(f"FAILED: {error}", file=sys.stderr)
        sys.exit(1)
