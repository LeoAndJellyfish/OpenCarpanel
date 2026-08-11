import { mkdtemp, readdir, readFile, rm } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { generateWebTypes } from "./generate-web-types.mjs";

const workspaceRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const committedDirectory = path.join(workspaceRoot, "web", "widget-sdk", "src", "generated");

async function files(directory) {
  return (await readdir(directory))
    .filter((entry) => entry.endsWith(".ts"))
    .sort((left, right) => left.localeCompare(right));
}

const temporaryDirectory = await mkdtemp(path.join(os.tmpdir(), "opencarpanel-web-types-"));
try {
  await generateWebTypes(temporaryDirectory);
  const generatedFiles = await files(temporaryDirectory);
  const committedFiles = await files(committedDirectory);
  if (JSON.stringify(generatedFiles) !== JSON.stringify(committedFiles)) {
    throw new Error(
      `generated file list differs: expected ${generatedFiles.join(", ")}; found ${committedFiles.join(", ")}`,
    );
  }

  for (const file of generatedFiles) {
    const generated = await readFile(path.join(temporaryDirectory, file), "utf8");
    const committed = await readFile(path.join(committedDirectory, file), "utf8");
    if (generated !== committed) {
      throw new Error(`${file} has drifted; run npm run generate:web-types`);
    }
  }
} finally {
  const resolvedTemporary = path.resolve(temporaryDirectory);
  const resolvedRoot = `${path.resolve(os.tmpdir())}${path.sep}`;
  if (resolvedTemporary.startsWith(resolvedRoot)) {
    await rm(resolvedTemporary, { recursive: true, force: true });
  }
}
