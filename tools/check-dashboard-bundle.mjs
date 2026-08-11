import { gzipSync } from "node:zlib";
import { readdir, readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const workspaceRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const outputDirectory = path.join(workspaceRoot, "web", "dashboard", "dist");
const assetDirectory = path.join(outputDirectory, "assets");
const assets = await readdir(assetDirectory);

const initialJavaScript = oneAsset(assets, /^index-.*\.js$/, "initial JavaScript");
const editorJavaScript = oneAsset(assets, /^edit-.*\.js$/, "editor JavaScript");
const initialCss = oneAsset(assets, /^index-.*\.css$/, "initial CSS");
const editorCss = oneAsset(assets, /^edit-.*\.css$/, "editor CSS");
const initialBytes = await readFile(path.join(assetDirectory, initialJavaScript));
const initialSource = initialBytes.toString("utf8");
const html = await readFile(path.join(outputDirectory, "index.html"), "utf8");

if (html.includes("edit-")) {
  throw new Error("editor assets are eagerly referenced by the driving HTML");
}
for (const editorMarker of ["editor-shell", "Revision conflict", "layout-draft"]) {
  if (initialSource.includes(editorMarker)) {
    throw new Error(`editor marker ${JSON.stringify(editorMarker)} leaked into the driving chunk`);
  }
}

const rawLimit = 64 * 1024;
const gzipLimit = 24 * 1024;
const gzipBytes = gzipSync(initialBytes).byteLength;
if (initialBytes.byteLength > rawLimit || gzipBytes > gzipLimit) {
  throw new Error(
    `driving JavaScript exceeds budget: ${initialBytes.byteLength} raw / ${gzipBytes} gzip bytes`,
  );
}

process.stdout.write(
  `dashboard bundle budgets passed: ${initialJavaScript} ${initialBytes.byteLength} B raw / ${gzipBytes} B gzip; lazy ${editorJavaScript}, ${initialCss}, ${editorCss}\n`,
);

function oneAsset(entries, pattern, description) {
  const matches = entries.filter((entry) => pattern.test(entry));
  if (matches.length !== 1) {
    throw new Error(`expected one ${description} asset, found ${matches.join(", ") || "none"}`);
  }
  return matches[0];
}
