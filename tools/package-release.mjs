import {
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
  process.platform === "win32" ? "opencarpanel-host.exe" : "opencarpanel-host",
);
const dashboardIndex = path.join(projectRoot, "web", "dashboard", "dist", "index.html");

if (!existsSync(executableSource)) {
  throw new Error(`Release Host is missing at ${executableSource}.`);
}
if (!existsSync(dashboardIndex)) {
  throw new Error("Dashboard build is missing; run npm run build:host first.");
}
if (statSync(dashboardIndex).mtimeMs > statSync(executableSource).mtimeMs) {
  throw new Error("Dashboard is newer than the Host binary; rebuild before packaging.");
}

const releaseRoot = path.resolve(projectRoot, "dist", "release");
const packageDirectory = path.resolve(
  releaseRoot,
  `OpenCarpanel-${packageMetadata.version}-${targetName}`,
);
if (!packageDirectory.startsWith(`${releaseRoot}${path.sep}`)) {
  throw new Error("Resolved release package escaped the release output directory.");
}
rmSync(packageDirectory, { recursive: true, force: true });
mkdirSync(packageDirectory, { recursive: true });

const executableName = process.platform === "win32" ? "OpenCarpanel.exe" : "OpenCarpanel";
copyFileSync(executableSource, path.join(packageDirectory, executableName));
copyFileSync(path.join(projectRoot, "LICENSE"), path.join(packageDirectory, "LICENSE"));
copyFileSync(path.join(projectRoot, "README.md"), path.join(packageDirectory, "README.md"));
cpSync(path.join(projectRoot, "docs"), path.join(packageDirectory, "docs"), {
  recursive: true,
});
cpSync(
  path.join(projectRoot, "docs", "protocols"),
  path.join(packageDirectory, "protocols"),
  { recursive: true },
);
copyFileSync(
  path.join(projectRoot, "docs", "quickstart-f1-24.md"),
  path.join(packageDirectory, "QUICKSTART.zh-CN.md"),
);
writeFileSync(
  path.join(packageDirectory, "build-info.json"),
  `${JSON.stringify(
    {
      schemaVersion: 1,
      application: "OpenCarpanel",
      version: packageMetadata.version,
      target: targetName,
      executable: executableName,
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
  throw new Error(`OpenCarpanel release packaging does not support ${value}.`);
}
