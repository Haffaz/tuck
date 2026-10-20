// Keeps the app version identical everywhere it is recorded.
//   node scripts/version.mjs 0.2.0     set the version
//   node scripts/version.mjs --check   print it; fail if the files disagree
//   node scripts/version.mjs --check v0.2.0   also fail unless it matches a tag
import { readFileSync, writeFileSync } from "node:fs";

const SEMVER = /^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?$/;
const PACKAGE_LINE = /(\[\[package\]\]\nname = "tuck"\nversion = ")([^"]+)(")/;
const CARGO_LINE = /^(version = ")([^"]+)(")/m;

const json = (path) => JSON.parse(readFileSync(path, "utf8"));
const writeJson = (path, value) =>
  writeFileSync(path, JSON.stringify(value, null, 2) + "\n");

const files = {
  "package.json": {
    read: () => json("package.json").version,
    write: (v) =>
      writeJson("package.json", { ...json("package.json"), version: v }),
  },
  "package-lock.json": {
    read: () => json("package-lock.json").version,
    write: (v) => {
      const lock = json("package-lock.json");
      const root = { ...lock.packages[""], version: v };
      writeJson("package-lock.json", {
        ...lock,
        version: v,
        packages: { ...lock.packages, "": root },
      });
    },
  },
  "src-tauri/tauri.conf.json": {
    read: () => json("src-tauri/tauri.conf.json").version,
    write: (v) =>
      writeJson("src-tauri/tauri.conf.json", {
        ...json("src-tauri/tauri.conf.json"),
        version: v,
      }),
  },
  "src-tauri/Cargo.toml": {
    read: () =>
      readFileSync("src-tauri/Cargo.toml", "utf8").match(CARGO_LINE)?.[2],
    write: (v) => replaceIn("src-tauri/Cargo.toml", CARGO_LINE, v),
  },
  "src-tauri/Cargo.lock": {
    read: () =>
      readFileSync("src-tauri/Cargo.lock", "utf8").match(PACKAGE_LINE)?.[2],
    write: (v) => replaceIn("src-tauri/Cargo.lock", PACKAGE_LINE, v),
  },
};

function replaceIn(path, pattern, version) {
  const text = readFileSync(path, "utf8");
  if (!pattern.test(text)) throw new Error(`No version found in ${path}`);
  writeFileSync(path, text.replace(pattern, `$1${version}$3`));
}

function check(tag) {
  const versions = Object.entries(files).map(([path, f]) => [path, f.read()]);
  const distinct = new Set(versions.map(([, v]) => v));
  if (distinct.size !== 1) {
    versions.forEach(([path, v]) => console.error(`${path}: ${v}`));
    throw new Error("Versions differ. Run: node scripts/version.mjs <version>");
  }
  const [version] = distinct;
  if (tag && tag !== `v${version}`)
    throw new Error(`Tag ${tag} does not match version ${version}`);
  console.log(version);
}

const [arg, tag] = process.argv.slice(2);
try {
  if (arg === "--check") check(tag);
  else if (arg && SEMVER.test(arg)) {
    Object.values(files).forEach((f) => f.write(arg));
    check();
  } else
    throw new Error("Usage: node scripts/version.mjs <x.y.z> | --check [tag]");
} catch (error) {
  console.error(error.message);
  process.exit(1);
}
