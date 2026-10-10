"""Build an evaluation-only Ollama transport around the unchanged filing pipeline."""
import argparse
from pathlib import Path
import shutil
import subprocess

parser = argparse.ArgumentParser()
parser.add_argument("output", type=Path)
args = parser.parse_args()
repo = Path(__file__).resolve().parents[2]
support = (repo / "src-tauri/swift/TuckModelSupport.swift").read_text()
start = support.index("@available(macOS 26.0, *)\nfunc generate(")
end = support.index("// A second opinion", start)
support = support[:start] + support[end:]
args.output.mkdir(parents=True, exist_ok=True)
(args.output / "TuckModelSupport.swift").write_text(support)
shutil.copy2(repo / "src-tauri/resources/tuck-reader", args.output / "tuck-reader")
subprocess.run(["swiftc", "-parse-as-library", "-O",
    str(repo / "src-tauri/swift/TuckApple.swift"),
    str(args.output / "TuckModelSupport.swift"),
    str(Path(__file__).with_name("OllamaTransport.swift")),
    "-o", str(args.output / "tuck-apple")], check=True)
