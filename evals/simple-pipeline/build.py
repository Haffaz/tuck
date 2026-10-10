"""Compile the candidate beside the production helper, without installing it."""
from pathlib import Path
import argparse
import re
import shutil
import subprocess

parser = argparse.ArgumentParser()
parser.add_argument("output", type=Path)
parser.add_argument("--variant", choices=["v1", "v2", "v3"], default="v3")
args = parser.parse_args()
repo = Path(__file__).resolve().parents[2]
source = repo / "src-tauri/swift/TuckApple.swift"
text = source.read_text()
prefix = text[:text.index('        var stage = "input"')]
prefix = re.sub(r'"filing-v\d+"', f'"single-decision-{args.variant}"', prefix)
main = prefix + r'''        do {
            let input = try JSONDecoder().decode(Input.self, from: data)
            emit(["reply": try await simpleFiling(input, model: model)])
        } catch {
            emit(["error": "Apple Intelligence could not complete filing (\(errorCode(error))). Choose the destination manually."])
        }
    }
}
'''
args.output.mkdir(parents=True, exist_ok=True)
(args.output / "TuckApple.swift").write_text(main)
(args.output / "Input.swift").write_text(text[:text.index("@main struct TuckApple")])
variant = Path(__file__).with_name("TuckSimple.swift" if args.variant == "v3" else f"TuckSimple-{args.variant}.swift.txt")
shutil.copy2(variant, args.output / "TuckSimple.swift")
shutil.copy2(repo / "src-tauri/resources/tuck-reader", args.output / "tuck-reader")
subprocess.run(["swiftc", "-parse-as-library", "-O", str(args.output / "TuckApple.swift"),
    str(repo / "src-tauri/swift/TuckModelSupport.swift"), str(args.output / "TuckSimple.swift"),
    "-o", str(args.output / "tuck-apple")], check=True)
