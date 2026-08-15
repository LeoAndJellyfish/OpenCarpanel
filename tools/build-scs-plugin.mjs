import { existsSync, rmSync } from "node:fs";
import path from "node:path";
import process from "node:process";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const projectRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const sourceDirectory = path.join(projectRoot, "plugins", "scs-telemetry-bridge");
const targetDirectory = path.join(projectRoot, "target");
const buildDirectory = path.join(targetDirectory, "scs-plugin-build");
const stageDirectory = path.join(targetDirectory, "scs-plugin-package");

if (!stageDirectory.startsWith(`${targetDirectory}${path.sep}`)) {
  throw new Error("Resolved SCS plugin stage escaped the Cargo target directory.");
}

const cmake = findCmake();
const ctest = process.platform === "win32"
  ? path.join(path.dirname(cmake), "ctest.exe")
  : "ctest";
const platformArguments = process.platform === "win32"
  ? ["-A", "x64"]
  : process.platform === "darwin"
    // SCS SDK 1.14 only defines the Intel ABI on macOS. This dylib is loaded
    // by the game process, not by the native Tauri desktop process.
    ? ["-DCMAKE_OSX_ARCHITECTURES=x86_64"]
    : [];
const buildTypeArguments = process.platform === "win32"
  ? []
  : ["-DCMAKE_BUILD_TYPE=Release"];

run(cmake, [
  "-S",
  sourceDirectory,
  "-B",
  buildDirectory,
  "-DBUILD_TESTING=ON",
  ...buildTypeArguments,
  ...platformArguments,
]);
run(cmake, ["--build", buildDirectory, "--config", "Release", "--parallel"]);
run(ctest, [
  "--test-dir",
  buildDirectory,
  "-C",
  "Release",
  "--output-on-failure",
]);

rmSync(stageDirectory, { recursive: true, force: true });
run(cmake, [
  "--install",
  buildDirectory,
  "--config",
  "Release",
  "--prefix",
  stageDirectory,
]);

if (process.platform === "darwin") {
  verifyMacosGameAbi();
}

process.stdout.write(`SCS telemetry plugin staged at ${stageDirectory}\n`);

function verifyMacosGameAbi() {
  const plugin = path.join(stageDirectory, "opensimdash-scs-telemetry.dylib");
  const result = spawnSync("file", [plugin], { encoding: "utf8" });
  if (result.error) {
    throw result.error;
  }
  if (result.status !== 0 || !result.stdout.includes("x86_64")) {
    throw new Error(
      `Expected the macOS SCS game plugin to be x86_64, got: ${result.stdout || result.stderr}`,
    );
  }
  process.stdout.write("Verified macOS SCS game plugin ABI: x86_64\n");
}

function findCmake() {
  const executable = process.platform === "win32" ? "cmake.exe" : "cmake";
  const candidates = [executable];
  if (process.platform === "win32") {
    const programFilesX86 = process.env["ProgramFiles(x86)"];
    const programFiles = process.env.ProgramFiles;
    if (programFiles) {
      candidates.push(path.join(programFiles, "CMake", "bin", executable));
    }
    if (programFilesX86) {
      for (const edition of ["BuildTools", "Community", "Professional", "Enterprise"]) {
        candidates.push(
          path.join(
            programFilesX86,
            "Microsoft Visual Studio",
            "2022",
            edition,
            "Common7",
            "IDE",
            "CommonExtensions",
            "Microsoft",
            "CMake",
            "CMake",
            "bin",
            executable,
          ),
        );
      }
    }
  }

  for (const candidate of candidates) {
    if (candidate !== executable && !existsSync(candidate)) {
      continue;
    }
    const probe = spawnSync(candidate, ["--version"], { stdio: "ignore" });
    if (probe.status === 0) {
      return candidate;
    }
  }
  throw new Error("CMake 3.20+ is required to build the SCS telemetry plugin.");
}

function run(command, arguments_) {
  const result = spawnSync(command, arguments_, {
    cwd: projectRoot,
    stdio: "inherit",
  });
  if (result.error) {
    throw result.error;
  }
  if (result.status !== 0) {
    throw new Error(`${command} exited with status ${result.status}.`);
  }
}
