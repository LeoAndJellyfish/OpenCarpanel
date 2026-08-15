import { readFileSync } from "node:fs";
import path from "node:path";
import process from "node:process";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const projectRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const listing = spawnSync(
  "git",
  ["ls-files", "--cached", "--others", "--exclude-standard", "-z"],
  { cwd: projectRoot, encoding: null },
);
if (listing.error) {
  throw listing.error;
}
if (listing.status !== 0) {
  throw new Error(`git ls-files failed with status ${listing.status}.`);
}

const trackedFiles = listing.stdout
  .toString("utf8")
  .split("\0")
  .filter(Boolean);
const retiredProductToken = ["car", "panel"].join("");
const retiredPrefix = ["o", "c", "p"].join("");
const forbiddenPath = new RegExp(`${retiredProductToken}|${retiredPrefix}`, "iu");
const forbiddenText = [
  { label: "former product name", pattern: new RegExp(retiredProductToken, "iu") },
  {
    label: "former plugin extension or ABI prefix",
    pattern: new RegExp(`(?:\\.${retiredPrefix}-plugin|\\b${retiredPrefix}(?:\\b|_|[0-9]))`, "iu"),
  },
  {
    label: "former protocol magic encoded as hexadecimal bytes",
    pattern: new RegExp(["0x4f", "0x43", "0x50"].join("\\s*,\\s*"), "iu"),
  },
  {
    label: "former protocol magic encoded as decimal bytes",
    pattern: new RegExp(
      `(?:^|\\D)${["O", "C", "P"].map((value) => value.charCodeAt(0)).join("\\s*,\\s*")}(?:\\D|$)`,
      "u",
    ),
  },
];
const failures = [];

for (const relativePath of trackedFiles) {
  if (forbiddenPath.test(relativePath)) {
    failures.push(`${relativePath}: filename contains a retired brand token`);
  }
  const bytes = readFileSync(path.join(projectRoot, relativePath));
  if (bytes.includes(0)) {
    continue;
  }
  const text = bytes.toString("utf8");
  for (const { label, pattern } of forbiddenText) {
    if (pattern.test(text)) {
      failures.push(`${relativePath}: contains ${label}`);
    }
  }
}

const requiredIdentity = [
  ["package.json", /"name"\s*:\s*"opensimdash"/u],
  ["apps/desktop/src-tauri/tauri.conf.json", /"productName"\s*:\s*"OpenSimDash"/u],
  ["crates/game-plugin-runtime/src/package.rs", /PLUGIN_PACKAGE_EXTENSION:\s*&str\s*=\s*"osd-plugin"/u],
  ["crates/game-plugin-sdk/src/lib.rs", /fn osd_plugin_abi_version/u],
  ["crates/adapter-scs/src/protocol.rs", /b"OSD\\0"/u],
];
for (const [relativePath, pattern] of requiredIdentity) {
  const text = readFileSync(path.join(projectRoot, relativePath), "utf8");
  if (!pattern.test(text)) {
    failures.push(`${relativePath}: required OpenSimDash identity marker is missing`);
  }
}

if (failures.length > 0) {
  throw new Error(`Brand audit failed:\n${failures.join("\n")}`);
}

process.stdout.write(`OpenSimDash brand audit passed across ${trackedFiles.length} files.\n`);
