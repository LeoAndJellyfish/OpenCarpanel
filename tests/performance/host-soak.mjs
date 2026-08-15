import { spawn } from "node:child_process";
import dgram from "node:dgram";
import { existsSync } from "node:fs";
import { fileURLToPath } from "node:url";
import path from "node:path";

const projectRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const defaults = {
  durationSeconds: 2 * 60 * 60,
  clients: 4,
  hz: 60,
  sampleIntervalSeconds: 60,
  binary: path.join(
    projectRoot,
    "target",
    "release",
    process.platform === "win32" ? "opensimdash-host.exe" : "opensimdash-host",
  ),
};

const options = parseArguments(process.argv.slice(2));
let host;
let udp;
let sendTimer;
let rssTimer;
const sockets = [];

try {
  if (!existsSync(options.binary)) {
    throw new Error(`Host binary not found at ${options.binary}; build the release Host first.`);
  }
  host = spawn(options.binary, [], {
    cwd: projectRoot,
    env: { ...process.env, RUST_LOG: "warn" },
    stdio: ["ignore", "pipe", "pipe"],
    windowsHide: true,
  });
  host.stderr.setEncoding("utf8");
  let hostErrors = "";
  host.stderr.on("data", (chunk) => {
    hostErrors = `${hostErrors}${chunk}`.slice(-8_192);
  });

  const pairing = await waitForPairingUrl(host);
  const websocketUrl = `ws://127.0.0.1:${pairing.port}/api/v1/ws`;
  const first = await openDashboard(websocketUrl, { pairingToken: pairing.token });
  sockets.push(first);
  if (!first.session) {
    throw new Error("Host did not issue a reusable device session.");
  }
  const resumed = await Promise.all(
    Array.from({ length: options.clients - 1 }, () =>
      openDashboard(websocketUrl, { deviceSession: first.session }),
    ),
  );
  sockets.push(...resumed);

  udp = dgram.createSocket("udp4");
  let frame = 0;
  let packetsSent = 0;
  const telemetryStartedAt = performance.now();
  sendTimer = setInterval(() => {
    const expectedPackets = Math.floor(
      ((performance.now() - telemetryStartedAt) * options.hz) / 1_000,
    );
    while (packetsSent < expectedPackets) {
      frame = (frame + 1) >>> 0;
      packetsSent += 1;
      udp.send(syntheticPacket(frame), 20_777, "127.0.0.1");
    }
  }, 4);

  const rssSamples = [];
  let rssSamplePromise;
  const sampleRss = () => {
    if (rssSamplePromise) {
      return rssSamplePromise;
    }
    rssSamplePromise = readRssBytes(host.pid)
      .then((rss) => {
        if (rss !== undefined) {
          rssSamples.push(rss);
        }
      })
      .finally(() => {
        rssSamplePromise = undefined;
      });
    return rssSamplePromise;
  };
  await sampleRss();
  rssTimer = setInterval(() => void sampleRss(), options.sampleIntervalSeconds * 1_000);

  await Promise.race([
    delay(options.durationSeconds * 1_000),
    new Promise((_, reject) => {
      host.once("exit", (code, signal) =>
        reject(
          new Error(
            `Host exited during soak (code=${String(code)}, signal=${String(signal)}).\n${hostErrors}`,
          ),
        ),
      );
    }),
  ]);
  clearInterval(sendTimer);
  sendTimer = undefined;
  await delay(100);
  await sampleRss();

  const diagnosticsResponse = await fetch(
    `http://127.0.0.1:${pairing.port}/api/v1/diagnostics`,
  );
  if (!diagnosticsResponse.ok) {
    throw new Error(`Diagnostics returned HTTP ${diagnosticsResponse.status}.`);
  }
  const diagnostics = await diagnosticsResponse.json();
  const nominalPackets = Math.floor(options.durationSeconds * options.hz);
  if (packetsSent < Math.floor(nominalPackets * 0.95)) {
    throw new Error(
      `Load generator sent only ${packetsSent} of approximately ${nominalPackets} packets.`,
    );
  }
  const minimumPackets = Math.floor(packetsSent * 0.98);
  if (diagnostics.telemetry.packetsReceived < minimumPackets) {
    throw new Error(
      `Host received ${diagnostics.telemetry.packetsReceived} of ${packetsSent} sent packets; expected at least ${minimumPackets}.`,
    );
  }
  if (diagnostics.telemetry.packetErrors !== 0) {
    throw new Error(`Host reported ${diagnostics.telemetry.packetErrors} packet errors.`);
  }
  for (const [index, client] of sockets.entries()) {
    if (client.closed || client.snapshots === 0) {
      throw new Error(
        `Dashboard client ${index + 1} did not remain live (snapshots=${client.snapshots}).`,
      );
    }
  }

  const memory = summarizeMemory(rssSamples);
  if (memory.growthBytes > 16 * 1024 * 1024) {
    throw new Error(
      `Host RSS grew by ${formatMiB(memory.growthBytes)} MiB; tolerance is 16 MiB.`,
    );
  }

  console.log("OpenSimDash Host soak passed");
  console.log(
    `duration=${options.durationSeconds}s rate=${options.hz}Hz sent=${packetsSent} clients=${options.clients}`,
  );
  console.log(`clientSnapshots=${sockets.map((client) => client.snapshots).join(",")}`);
  if (rssSamples.length > 0) {
    console.log(
      `rssMiB start=${formatMiB(memory.startBytes)} end=${formatMiB(memory.endBytes)} peak=${formatMiB(memory.peakBytes)} growth=${formatMiB(memory.growthBytes)}`,
    );
  } else {
    console.log("rssMiB unavailable on this platform");
  }
} catch (error) {
  console.error(error instanceof Error ? error.stack : error);
  process.exitCode = 1;
} finally {
  clearInterval(sendTimer);
  clearInterval(rssTimer);
  if (udp) {
    await new Promise((resolve) => udp.close(resolve));
  }
  for (const client of sockets) {
    client.socket.close(1000, "soak_complete");
  }
  if (host && host.exitCode === null) {
    host.kill();
    await Promise.race([new Promise((resolve) => host.once("exit", resolve)), delay(2_000)]);
    if (host.exitCode === null) {
      host.kill("SIGKILL");
    }
  }
}

function parseArguments(arguments_) {
  if (arguments_.includes("--help")) {
    console.log(`Usage: node tests/performance/host-soak.mjs [options]

Options:
  --duration-seconds N       Soak duration (default: 7200)
  --clients N                Concurrent WebSocket dashboards (default: 4, max: 8)
  --hz N                     Synthetic F1 packet rate (default: 60)
  --sample-interval-seconds N RSS sample interval (default: 60)
  --binary PATH              Release Host executable`);
    process.exit(0);
  }
  const parsed = { ...defaults };
  const keys = new Map([
    ["--duration-seconds", "durationSeconds"],
    ["--clients", "clients"],
    ["--hz", "hz"],
    ["--sample-interval-seconds", "sampleIntervalSeconds"],
    ["--binary", "binary"],
  ]);
  for (let index = 0; index < arguments_.length; index += 2) {
    const flag = arguments_[index];
    const value = arguments_[index + 1];
    const key = keys.get(flag);
    if (!key || value === undefined) {
      throw new Error(`Unknown or incomplete option ${String(flag)}.`);
    }
    parsed[key] = key === "binary" ? path.resolve(value) : Number(value);
  }
  for (const key of ["durationSeconds", "clients", "hz", "sampleIntervalSeconds"]) {
    if (!Number.isFinite(parsed[key]) || parsed[key] <= 0) {
      throw new Error(`${key} must be a positive number.`);
    }
  }
  if (!Number.isInteger(parsed.clients) || parsed.clients > 8) {
    throw new Error("clients must be an integer from 1 through 8.");
  }
  return parsed;
}

function waitForPairingUrl(child) {
  return new Promise((resolve, reject) => {
    let output = "";
    const timeout = setTimeout(() => reject(new Error("Timed out waiting for Host pairing URL.")), 15_000);
    child.stdout.setEncoding("utf8");
    const onData = (chunk) => {
      output = `${output}${chunk}`.slice(-64 * 1024);
      const match = output.match(/http:\/\/[^\s/:]+:(\d+)\/#pair=([A-Za-z0-9_-]+)/);
      if (match) {
        clearTimeout(timeout);
        child.stdout.off("data", onData);
        child.stdout.resume();
        resolve({ port: Number(match[1]), token: match[2] });
      }
    };
    child.stdout.on("data", onData);
    child.once("exit", (code) => {
      clearTimeout(timeout);
      reject(new Error(`Host exited before pairing URL was ready (code=${String(code)}).`));
    });
  });
}

function openDashboard(url, credential) {
  return new Promise((resolve, reject) => {
    const socket = new WebSocket(url);
    const state = { socket, session: undefined, snapshots: 0, closed: false };
    const timeout = setTimeout(() => {
      socket.close();
      reject(new Error("Timed out opening a dashboard WebSocket."));
    }, 5_000);
    socket.addEventListener("open", () => {
      socket.send(
        JSON.stringify({
          v: 1,
          type: "hello",
          ...credential,
          lastEventSeq: 0,
          snapshotHz: 60,
        }),
      );
    });
    socket.addEventListener("message", (event) => {
      if (typeof event.data !== "string") {
        return;
      }
      const message = JSON.parse(event.data);
      if (message.type === "hello") {
        state.session = message.deviceSession;
        clearTimeout(timeout);
        resolve(state);
      } else if (message.type === "snapshot") {
        state.snapshots += 1;
      }
    });
    socket.addEventListener("error", () => {
      clearTimeout(timeout);
      reject(new Error("Dashboard WebSocket failed."));
    });
    socket.addEventListener("close", () => {
      state.closed = true;
    });
  });
}

function syntheticPacket(frame) {
  const packet = Buffer.alloc(1_352);
  packet.writeUInt16LE(2_024, 0);
  packet.set([24, 1, 0, 1, 6], 2);
  packet.writeBigUInt64LE(0x0102_0304_0506_0708n, 7);
  packet.writeFloatLE(1, 15);
  packet.writeUInt32LE(frame, 19);
  packet.writeUInt32LE(frame, 23);
  packet[27] = 0;
  packet[28] = 255;
  const player = 29;
  packet.writeUInt16LE(280 + (frame % 40), player);
  packet.writeFloatLE(0.8, player + 2);
  packet.writeFloatLE(0.1, player + 10);
  packet.writeInt8(7, player + 15);
  packet.writeUInt16LE(11_500, player + 16);
  packet[player + 18] = 1;
  packet[player + 19] = 80;
  return packet;
}

async function readRssBytes(pid) {
  if (!Number.isInteger(pid)) {
    return undefined;
  }
  if (process.platform === "win32") {
    const output = await captureOutput(
      "powershell.exe",
      ["-NoProfile", "-Command", `(Get-Process -Id ${pid}).WorkingSet64`],
    );
    const value = Number(output.trim());
    return Number.isFinite(value) ? value : undefined;
  }
  const output = await captureOutput("ps", ["-o", "rss=", "-p", String(pid)]);
  const kibibytes = Number(output.trim());
  return Number.isFinite(kibibytes) ? kibibytes * 1_024 : undefined;
}

function captureOutput(command, arguments_) {
  return new Promise((resolve) => {
    const child = spawn(command, arguments_, {
      stdio: ["ignore", "pipe", "ignore"],
      windowsHide: true,
    });
    let output = "";
    const timeout = setTimeout(() => child.kill(), 5_000);
    child.stdout.setEncoding("utf8");
    child.stdout.on("data", (chunk) => {
      output += chunk;
    });
    child.once("error", () => {
      clearTimeout(timeout);
      resolve("");
    });
    child.once("exit", () => {
      clearTimeout(timeout);
      resolve(output);
    });
  });
}

function summarizeMemory(samples) {
  if (samples.length === 0) {
    return { startBytes: 0, endBytes: 0, peakBytes: 0, growthBytes: 0 };
  }
  const startBytes = samples[0];
  const endBytes = samples.at(-1);
  return {
    startBytes,
    endBytes,
    peakBytes: Math.max(...samples),
    growthBytes: Math.max(0, endBytes - startBytes),
  };
}

function formatMiB(bytes) {
  return (bytes / 1024 / 1024).toFixed(2);
}

function delay(milliseconds) {
  return new Promise((resolve) => setTimeout(resolve, milliseconds));
}
