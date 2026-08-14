import { spawn } from "node:child_process";
import { createSocket } from "node:dgram";
import { once } from "node:events";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync } from "node:fs";
import { createServer } from "node:net";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

const projectRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const packageMetadata = JSON.parse(
  readFileSync(path.join(projectRoot, "package.json"), "utf8"),
);
const platform = platformName(process.platform);
const targetName = `${platform}-${process.arch}`;
const packageDirectory = path.join(
  projectRoot,
  "dist",
  "release",
  `OpenCarpanel-${packageMetadata.version}-${targetName}`,
);
const executable = path.join(
  packageDirectory,
  process.platform === "win32" ? "OpenCarpanel.exe" : "OpenCarpanel",
);

if (!existsSync(executable)) {
  throw new Error(`Packaged Host is missing at ${executable}; run npm run package:host first.`);
}

const targetDirectory = path.join(projectRoot, "target");
mkdirSync(targetDirectory, { recursive: true });
const dataDirectory = mkdtempSync(path.join(targetDirectory, "package-smoke-"));
const [httpPort, udpPort] = await Promise.all([availableTcpPort(), availableUdpPort()]);
const host = spawn(executable, [], {
  cwd: packageDirectory,
  env: {
    ...process.env,
    OPENCARPANEL_DATA_DIR: dataDirectory,
    OPENCARPANEL_RUNTIME_DIR: path.join(dataDirectory, "runtime"),
    OPENCARPANEL_GAME: "auto",
    OPENCARPANEL_HTTP_BIND: `127.0.0.1:${httpPort}`,
    OPENCARPANEL_UDP_BIND: `127.0.0.1:${udpPort}`,
  },
  stdio: ["ignore", "pipe", "pipe"],
  windowsHide: true,
});
const udp = createSocket("udp4");
let stdout = "";
let stderr = "";
let exit = undefined;
let succeeded = false;

host.stdout.setEncoding("utf8");
host.stderr.setEncoding("utf8");
host.stdout.on("data", (chunk) => {
  stdout = keepTail(stdout + chunk);
});
host.stderr.on("data", (chunk) => {
  stderr = keepTail(stderr + chunk);
});
host.on("exit", (code, signal) => {
  exit = { code, signal };
});

try {
  const health = await waitForJson("/api/v1/health", (body) => body.status === "ok", 10_000);
  const expectedAdapters = ["f1-24", "f1-25", "ets2", "ats"];
  if (JSON.stringify(health.supportedAdapters) !== JSON.stringify(expectedAdapters)) {
    throw new Error(`Unexpected supported adapters: ${JSON.stringify(health.supportedAdapters)}`);
  }

  const activated = [];
  const cases = [
    { packet: f1Packet(2024, 24), adapter: "f1-24", label: "f1-24/2024" },
    { packet: f1Packet(2025, 25), adapter: "f1-25", label: "f1-25/2025" },
    { packet: f1Packet(2026, 26), adapter: "f1-25", label: "f1-25/2026" },
    { packet: scsPacket(1), adapter: "ets2", label: "ets2" },
    { packet: scsPacket(2), adapter: "ats", label: "ats" },
  ];
  for (const [index, { packet, adapter, label }] of cases.entries()) {
    if (index > 0 && cases[index - 1].adapter !== adapter) {
      await delay(2_100);
    }
    await sendPacket(packet);
    const diagnostics = await waitForJson(
      "/api/v1/diagnostics",
      (body) => body.activeAdapter === adapter
        && body.telemetry.packetsReceived === index + 1,
      2_000,
    );
    activated.push(label);
  }

  const diagnostics = await requestJson("/api/v1/diagnostics");
  if (
    diagnostics.adapterSelection !== "auto"
    || diagnostics.telemetry.packetsReceived !== 5
    || diagnostics.telemetry.packetsRecognized !== 5
    || diagnostics.telemetry.packetErrors !== 0
  ) {
    throw new Error(`Unexpected packaged Host diagnostics: ${JSON.stringify(diagnostics)}`);
  }

  process.stdout.write(
    `Package smoke passed: ${activated.join(" -> ")}; `
      + `${diagnostics.telemetry.packetsRecognized}/5 packets recognized, 0 errors.\n`,
  );
  succeeded = true;
} catch (error) {
  const context = [
    `Packaged Host smoke failed: ${error instanceof Error ? error.message : String(error)}`,
    exit ? `Host exit: ${JSON.stringify(exit)}` : "Host was still running.",
    stdout ? `Host stdout tail:\n${stdout}` : "Host stdout was empty.",
    stderr ? `Host stderr tail:\n${stderr}` : "Host stderr was empty.",
    `Smoke data was retained at ${dataDirectory}`,
  ];
  throw new Error(context.join("\n"));
} finally {
  udp.close();
  if (exit === undefined) {
    const exitEvent = once(host, "exit");
    host.kill();
    await exitEvent;
  }
  if (succeeded) {
    const resolvedDataDirectory = path.resolve(dataDirectory);
    const safePrefix = `${path.resolve(targetDirectory)}${path.sep}package-smoke-`;
    if (!resolvedDataDirectory.startsWith(safePrefix)) {
      throw new Error(`Refusing to remove unexpected smoke path: ${resolvedDataDirectory}`);
    }
    rmSync(resolvedDataDirectory, { recursive: true });
  }
}

async function requestJson(route) {
  const response = await fetch(`http://127.0.0.1:${httpPort}${route}`, {
    signal: AbortSignal.timeout(1_000),
  });
  if (!response.ok) {
    throw new Error(`${route} returned HTTP ${response.status}`);
  }
  return response.json();
}

async function waitForJson(route, predicate, timeoutMs) {
  const deadline = Date.now() + timeoutMs;
  let lastError = undefined;
  while (Date.now() < deadline) {
    if (exit !== undefined) {
      throw new Error(`Host exited before ${route} was ready: ${JSON.stringify(exit)}`);
    }
    try {
      const body = await requestJson(route);
      if (predicate(body)) {
        return body;
      }
    } catch (error) {
      lastError = error;
    }
    await delay(50);
  }
  throw new Error(
    `${route} did not reach the expected state within ${timeoutMs} ms`
      + (lastError instanceof Error ? `: ${lastError.message}` : ""),
  );
}

function sendPacket(packet) {
  return new Promise((resolve, reject) => {
    udp.send(packet, udpPort, "127.0.0.1", (error) => {
      if (error) {
        reject(error);
      } else {
        resolve();
      }
    });
  });
}

function f1Packet(format, gameYear) {
  const seasonPack = format === 2026;
  const carDataLength = seasonPack ? 59 : 60;
  const playerIndex = seasonPack ? 23 : 0;
  const packet = Buffer.alloc(seasonPack ? 1_448 : 1_352);
  packet.writeUInt16LE(format, 0);
  packet.writeUInt8(gameYear, 2);
  packet.writeUInt8(1, 3);
  packet.writeUInt8(1, 5);
  packet.writeUInt8(6, 6);
  packet.writeBigUInt64LE(0x0102_0304_0506_0708n, 7);
  packet.writeFloatLE(1, 15);
  packet.writeUInt32LE(10, 19);
  packet.writeUInt32LE(12, 23);
  packet.writeUInt8(playerIndex, 27);
  packet.writeUInt8(255, 28);
  const player = 29 + playerIndex * carDataLength;
  packet.writeUInt16LE(240, player);
  packet.writeFloatLE(0.8, player + 2);
  packet.writeFloatLE(0.1, player + 10);
  packet.writeInt8(7, player + 15);
  packet.writeUInt16LE(11_000, player + 16);
  packet.writeUInt8(1, player + 18);
  return packet;
}

function scsPacket(gameId) {
  const packet = Buffer.alloc(188);
  Buffer.from([0x4f, 0x43, 0x50, 0]).copy(packet, 0);
  packet.writeUInt8(2, 4);
  packet.writeUInt8(gameId, 5);
  packet.writeBigUInt64LE(0x1112_1314_1516_1718n, 8);
  packet.writeUInt32LE(42, 16);
  packet.writeFloatLE(-20, 20);
  packet.writeFloatLE(1_300, 24);
  packet.writeFloatLE(2_500, 28);
  packet.writeInt32LE(6, 32);
  packet.writeFloatLE(0.75, 36);
  packet.writeFloatLE(0.1, 40);
  packet.writeFloatLE(12_500, 44);
  packet.writeFloatLE(600, 48);
  packet.writeFloatLE(25, 52);
  packet.writeFloatLE(275, 56);
  packet.writeFloatLE(400, 60);
  packet.writeFloatLE(900, 64);
  packet.writeUInt16LE(0x01d3, 68);
  packet.writeUInt16LE(0x000f, 70);
  packet.writeUInt32LE(11_000, 72);
  packet.writeUInt32LE(650, 76);
  packet.writeBigUInt64LE(85_000n, 80);
  packet.writeFloatLE(22_000, 88);
  packet.write("Medical Vaccines", 92, "utf8");
  packet.write("Hamburg", 124, "utf8");
  packet.write("Oslo", 156, "utf8");
  return packet;
}

function delay(milliseconds) {
  return new Promise((resolve) => setTimeout(resolve, milliseconds));
}

function keepTail(value) {
  return value.slice(-32_768);
}

function availableTcpPort() {
  return new Promise((resolve, reject) => {
    const server = createServer();
    server.unref();
    server.once("error", reject);
    server.listen(0, "127.0.0.1", () => {
      const address = server.address();
      if (!address || typeof address === "string") {
        server.close();
        reject(new Error("Could not allocate a temporary TCP port."));
        return;
      }
      server.close((error) => error ? reject(error) : resolve(address.port));
    });
  });
}

function availableUdpPort() {
  return new Promise((resolve, reject) => {
    const socket = createSocket("udp4");
    socket.once("error", reject);
    socket.bind(0, "127.0.0.1", () => {
      const address = socket.address();
      socket.close(() => resolve(address.port));
    });
  });
}

function platformName(value) {
  if (value === "win32") {
    return "windows";
  }
  if (value === "darwin") {
    return "macos";
  }
  throw new Error(`OpenCarpanel release smoke does not support ${value}.`);
}
