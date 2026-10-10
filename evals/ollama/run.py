"""Repeat the frozen comparison locally, one model at a time; never move documents."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time
import urllib.request

BASE = "http://127.0.0.1:11434"
MODELS = ["qwen3.5:4b", "qwen3.5:9b"]


def api(path, body=None):
    request = urllib.request.Request(BASE + path, data=None if body is None else json.dumps(body).encode(),
                                    headers={"Content-Type": "application/json"})
    with urllib.request.urlopen(request, timeout=180) as response:
        return json.load(response)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path, help="New report directory (must not already exist)")
    parser.add_argument("--models", nargs="+", choices=MODELS, default=MODELS)
    parser.add_argument("--apple", action="store_true", help="Also run a fresh Apple baseline after unloading Ollama")
    args = parser.parse_args()
    repo = Path(__file__).resolve().parents[2]
    tags = {m["name"]: m for m in api("/api/tags")["models"]}
    missing = [m for m in args.models if m not in tags]
    if missing:
        raise SystemExit("Download first: " + "; ".join("ollama pull " + m for m in missing))
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    paths = ["evals/simple-pipeline/comparison.json", "src-tauri/swift/TuckApple.swift",
             "src-tauri/swift/TuckModelSupport.swift", "evals/ollama/OllamaTransport.swift",
             "src-tauri/src/agent.rs", "src-tauri/src/apple.rs", "src-tauri/src/classifier.rs", "src-tauri/src/evaluation.rs"]
    manifest = {"ollama": api("/api/version"), "source_sha256": {
        p: hashlib.sha256((repo / p).read_bytes()).hexdigest() for p in paths}, "models": {}}
    def run(label, reader, model=None):
        env = os.environ.copy()
        env.update(TUCK_EVAL_READER=str(reader), TUCK_EVAL_FIXTURES=str(repo / paths[0]),
                   TUCK_EVAL_REPORT=str(output / (label + ".json")), TUCK_EVAL_LABEL=label)
        if model:
            env.update(TUCK_OLLAMA_MODEL=model, TUCK_OLLAMA_METRICS=str(output / (label + "-calls.jsonl")))
        with (output / (label + ".log")).open("w") as log:
            subprocess.run(["cargo", "test", "--manifest-path", "src-tauri/Cargo.toml",
                            "live_apple_accuracy_eval", "--", "--ignored", "--nocapture"],
                           cwd=repo, env=env, stdout=log, stderr=subprocess.STDOUT, check=True)
        print(label + ": " + json.dumps(json.loads((output / (label + ".json")).read_text())["summary"]), flush=True)
    with tempfile.TemporaryDirectory(prefix="tuck-ollama-", dir="/private/tmp") as helper:
        subprocess.run(["python3", str(Path(__file__).with_name("build.py")), helper], check=True)
        for model in args.models:
            # Keep only one benchmark model resident; leave unrelated Ollama sessions alone.
            for other in args.models:
                if other != model:
                    api("/api/generate", {"model": other, "keep_alive": 0})
            shown = api("/api/show", {"model": model})
            start = time.monotonic()
            warm = api("/api/chat", {"model": model, "messages": [{"role": "user", "content": "Reply OK."}],
                "stream": False, "think": False, "keep_alive": "10m", "options": {"temperature": 0, "num_ctx": 8192, "num_predict": 8}})
            manifest["models"][model] = {"tag": tags[model], "details": shown.get("details"),
                "capabilities": shown.get("capabilities"), "warmup_seconds": time.monotonic() - start,
                "warmup_load_nanoseconds": warm.get("load_duration"), "running": api("/api/ps")}
            (output / "manifest.json").write_text(json.dumps(manifest, indent=2))
            print("Running " + model, flush=True)
            try:
                run("filing-v6-ollama-" + model.replace(":", "-"), Path(helper) / "tuck-reader", model)
            finally:
                api("/api/generate", {"model": model, "keep_alive": 0})
        if args.apple:
            run("filing-v6-apple", repo / "src-tauri/resources/tuck-reader")


if __name__ == "__main__":
    main()
