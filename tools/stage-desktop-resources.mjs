import {
  copyFileSync,
  existsSync,
  mkdirSync,
  rmSync,
} from "node:fs";
import path from "node:path";
import process from "node:process";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const projectRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const tauriRoot = path.join(projectRoot, "apps", "desktop", "src-tauri");
const binaryDirectory = path.join(tauriRoot, "binaries");
const resourceDirectory = path.join(tauriRoot, "resources");
const targetDirectory = path.join(projectRoot, "target");

for (const generated of [binaryDirectory, resourceDirectory]) {
  if (!generated.startsWith(`${tauriRoot}${path.sep}`)) {
    throw new Error(`Desktop staging path escaped src-tauri: ${generated}`);
  }
  rmSync(generated, { recursive: true, force: true });
}

const triple = targetTriple();
const executableExtension = process.platform === "win32" ? ".exe" : "";
const releaseDirectory = process.env.CARGO_BUILD_TARGET
  ? path.join(targetDirectory, triple, "release")
  : path.join(targetDirectory, "release");
const hostSource = path.join(
  releaseDirectory,
  `opencarpanel-host${executableExtension}`,
);
const hostDestination = path.join(
  binaryDirectory,
  `opencarpanel-host-${triple}${executableExtension}`,
);

const pluginName = process.platform === "win32"
  ? "opencarpanel-scs-telemetry.dll"
  : process.platform === "darwin"
    ? "opencarpanel-scs-telemetry.dylib"
    : "opencarpanel-scs-telemetry.so";
const scsStage = path.join(targetDirectory, "scs-plugin-package");
const pluginSource = path.join(scsStage, pluginName);
const pluginDestinationDirectory = path.join(resourceDirectory, "plugins", "scs");

const required = [
  [hostSource, "release headless Host"],
  [pluginSource, "current-platform SCS bridge"],
  [path.join(scsStage, "README.md"), "SCS bridge guide"],
  [path.join(scsStage, "sdk_license.txt"), "SCS SDK license"],
  [path.join(projectRoot, "LICENSE"), "project license"],
  [path.join(projectRoot, "NOTICE"), "third-party notice"],
];
for (const [requiredPath, description] of required) {
  if (!existsSync(requiredPath)) {
    throw new Error(`Missing ${description}: ${requiredPath}`);
  }
}

mkdirSync(binaryDirectory, { recursive: true });
mkdirSync(pluginDestinationDirectory, { recursive: true });
mkdirSync(path.join(resourceDirectory, "docs"), { recursive: true });
copyFileSync(hostSource, hostDestination);
copyFileSync(pluginSource, path.join(pluginDestinationDirectory, pluginName));
copyFileSync(path.join(scsStage, "README.md"), path.join(pluginDestinationDirectory, "README.md"));
copyFileSync(
  path.join(scsStage, "sdk_license.txt"),
  path.join(pluginDestinationDirectory, "SCS-SDK-LICENSE.txt"),
);
copyFileSync(path.join(projectRoot, "LICENSE"), path.join(resourceDirectory, "LICENSE"));
copyFileSync(path.join(projectRoot, "NOTICE"), path.join(resourceDirectory, "NOTICE"));
for (const document of ["quickstart-multi-game.md", "quickstart-f1-24.md", "quickstart-f1-25.md", "quickstart-scs.md", "plugin-development.md"]) {
  copyFileSync(
    path.join(projectRoot, "docs", document),
    path.join(resourceDirectory, "docs", document),
  );
}

process.stdout.write(
  `Desktop resources staged for ${triple}: headless Host + ${pluginName}\n`,
);

function targetTriple() {
  if (process.env.CARGO_BUILD_TARGET) {
    return process.env.CARGO_BUILD_TARGET;
  }
  const probe = spawnSync("rustc", ["-vV"], { encoding: "utf8" });
  if (probe.status !== 0) {
    throw new Error("rustc -vV failed while resolving the Tauri sidecar triple.");
  }
  const hostLine = probe.stdout
    .split(/\r?\n/u)
    .find((line) => line.startsWith("host: "));
  if (!hostLine) {
    throw new Error("rustc did not report its host target triple.");
  }
  return hostLine.slice("host: ".length).trim();
}
