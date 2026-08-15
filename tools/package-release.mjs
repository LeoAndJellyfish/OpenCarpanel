import {
  chmodSync,
  copyFileSync,
  cpSync,
  existsSync,
  mkdirSync,
  readFileSync,
  rmSync,
  statSync,
  writeFileSync,
} from "node:fs";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

const projectRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const packageMetadata = JSON.parse(
  readFileSync(path.join(projectRoot, "package.json"), "utf8"),
);
const platform = platformName(process.platform);
const targetName = `${platform}-${process.arch}`;
const executableSource = path.join(
  projectRoot,
  "target",
  "release",
  process.platform === "win32" ? "opensimdash-host.exe" : "opensimdash-host",
);
const dashboardIndex = path.join(projectRoot, "web", "dashboard", "dist", "index.html");
const scsPluginStage = path.join(projectRoot, "target", "scs-plugin-package");
const scsPluginName = process.platform === "win32"
  ? "opensimdash-scs-telemetry.dll"
  : "opensimdash-scs-telemetry.dylib";
const scsPluginSource = path.join(scsPluginStage, scsPluginName);
const requiredPackageInputs = [
  [path.join(projectRoot, "LICENSE"), "project license"],
  [path.join(projectRoot, "NOTICE"), "third-party NOTICE"],
  [path.join(projectRoot, "assets", "readme", "hero.svg"), "README hero"],
  [path.join(projectRoot, "docs", "quickstart-multi-game.md"), "multi-game quick start"],
  [path.join(projectRoot, "docs", "quickstart-f1-24.md"), "F1 24 quick start"],
  [path.join(projectRoot, "docs", "quickstart-f1-25.md"), "F1 25 quick start"],
  [path.join(projectRoot, "docs", "quickstart-scs.md"), "SCS quick start"],
  [path.join(scsPluginStage, "README.md"), "SCS plugin installation guide"],
  [path.join(scsPluginStage, "sdk_license.txt"), "SCS SDK license"],
];

if (!existsSync(executableSource)) {
  throw new Error(`Release Host is missing at ${executableSource}.`);
}
if (!existsSync(dashboardIndex)) {
  throw new Error("Dashboard build is missing; run npm run build:host first.");
}
if (!existsSync(scsPluginSource)) {
  throw new Error(`SCS telemetry plugin is missing at ${scsPluginSource}.`);
}
for (const [requiredPath, description] of requiredPackageInputs) {
  if (!existsSync(requiredPath)) {
    throw new Error(`Required ${description} is missing at ${requiredPath}.`);
  }
}
if (statSync(dashboardIndex).mtimeMs > statSync(executableSource).mtimeMs) {
  throw new Error("Dashboard is newer than the Host binary; rebuild before packaging.");
}

const releaseRoot = path.resolve(projectRoot, "dist", "release");
const packageDirectory = path.resolve(
  releaseRoot,
  `OpenSimDash-${packageMetadata.version}-${targetName}`,
);
if (!packageDirectory.startsWith(`${releaseRoot}${path.sep}`)) {
  throw new Error("Resolved release package escaped the release output directory.");
}
rmSync(packageDirectory, { recursive: true, force: true });
mkdirSync(packageDirectory, { recursive: true });

const executableName = process.platform === "win32" ? "OpenSimDash.exe" : "OpenSimDash";
const executableDestination = path.join(packageDirectory, executableName);
copyFileSync(executableSource, executableDestination);
if (process.platform !== "win32") {
  chmodSync(executableDestination, 0o755);
}
copyFileSync(path.join(projectRoot, "LICENSE"), path.join(packageDirectory, "LICENSE"));
copyFileSync(path.join(projectRoot, "NOTICE"), path.join(packageDirectory, "NOTICE"));
copyFileSync(path.join(projectRoot, "README.md"), path.join(packageDirectory, "README.md"));
cpSync(
  path.join(projectRoot, "assets", "readme"),
  path.join(packageDirectory, "assets", "readme"),
  { recursive: true },
);
cpSync(path.join(projectRoot, "docs"), path.join(packageDirectory, "docs"), {
  recursive: true,
});
cpSync(
  path.join(projectRoot, "docs", "protocols"),
  path.join(packageDirectory, "protocols"),
  { recursive: true },
);
const scsPackageDirectory = path.join(packageDirectory, "plugins", "scs");
mkdirSync(scsPackageDirectory, { recursive: true });
copyFileSync(scsPluginSource, path.join(scsPackageDirectory, scsPluginName));
copyFileSync(
  path.join(scsPluginStage, "README.md"),
  path.join(scsPackageDirectory, "README.md"),
);
copyFileSync(
  path.join(scsPluginStage, "sdk_license.txt"),
  path.join(scsPackageDirectory, "SCS-SDK-LICENSE.txt"),
);
copyFileSync(
  path.join(projectRoot, "docs", "quickstart-multi-game.md"),
  path.join(packageDirectory, "QUICKSTART.zh-CN.md"),
);
copyFileSync(
  path.join(projectRoot, "docs", "quickstart-f1-24.md"),
  path.join(packageDirectory, "QUICKSTART-F1-24.zh-CN.md"),
);
copyFileSync(
  path.join(projectRoot, "docs", "quickstart-f1-25.md"),
  path.join(packageDirectory, "QUICKSTART-F1-25.zh-CN.md"),
);
copyFileSync(
  path.join(projectRoot, "docs", "quickstart-scs.md"),
  path.join(packageDirectory, "QUICKSTART-SCS.zh-CN.md"),
);
writeFileSync(
  path.join(packageDirectory, "build-info.json"),
  `${JSON.stringify(
    {
      schemaVersion: 1,
      application: "OpenSimDash",
      version: packageMetadata.version,
      target: targetName,
      executable: executableName,
      scsTelemetryPlugin: path.posix.join("plugins", "scs", scsPluginName),
      scsBridgeProtocolVersion: 1,
      signed: false,
    },
    undefined,
    2,
  )}\n`,
  "utf8",
);

console.log(`Release directory ready: ${packageDirectory}`);

function platformName(value) {
  if (value === "win32") {
    return "windows";
  }
  if (value === "darwin") {
    return "macos";
  }
  throw new Error(`OpenSimDash release packaging does not support ${value}.`);
}
