import { mkdir, readFile, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import { compile } from "json-schema-to-typescript";

const workspaceRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const defaultOutput = path.join(workspaceRoot, "web", "widget-sdk", "src", "generated");
const bannerComment = `/**
 * Generated from the committed OpenCarpanel JSON Schemas.
 * Do not edit by hand; run \`npm run generate:web-types\`.
 */`;
const schemas = [
  {
    source: path.join(workspaceRoot, "schemas", "protocol", "v1", "client-message.schema.json"),
    output: "client-message.ts",
  },
  {
    source: path.join(workspaceRoot, "schemas", "protocol", "v1", "server-message.schema.json"),
    output: "server-message.ts",
  },
];

function outputArgument(arguments_) {
  const outputIndex = arguments_.indexOf("--out");
  if (outputIndex === -1) {
    return defaultOutput;
  }
  const value = arguments_[outputIndex + 1];
  if (!value) {
    throw new Error("--out requires a directory");
  }
  return path.resolve(value);
}

function messageTypes(schema, source) {
  if (!Array.isArray(schema.oneOf)) {
    throw new Error(`${source} does not contain a oneOf message union`);
  }
  return schema.oneOf.map((variant) => {
    const type = variant?.properties?.type?.const;
    if (typeof type !== "string") {
      throw new Error(`${source} contains a message without a string type constant`);
    }
    return type;
  });
}

function protocolVersion(schema, source) {
  const version = schema?.properties?.v?.const;
  if (!Number.isSafeInteger(version)) {
    throw new Error(`${source} does not pin a safe integer protocol version`);
  }
  return version;
}

async function readSchema(source) {
  return JSON.parse(await readFile(source, "utf8"));
}

function quoteList(values) {
  return values.map((value) => JSON.stringify(value)).join(", ");
}

function normalizeRefSiblings(value) {
  if (Array.isArray(value)) {
    return value.map(normalizeRefSiblings);
  }
  if (value === null || typeof value !== "object") {
    return value;
  }

  const normalized = Object.fromEntries(
    Object.entries(value).map(([key, entry]) => [key, normalizeRefSiblings(entry)]),
  );
  if (typeof normalized.$ref !== "string" || Object.keys(normalized).length === 1) {
    return normalized;
  }

  const { $ref, ...siblings } = normalized;
  const annotationKeys = new Set([
    "$comment",
    "default",
    "deprecated",
    "description",
    "examples",
    "readOnly",
    "title",
    "writeOnly",
  ]);
  const annotations = Object.fromEntries(
    Object.entries(siblings).filter(([key]) => annotationKeys.has(key)),
  );
  const constraints = Object.fromEntries(
    Object.entries(siblings).filter(([key]) => !annotationKeys.has(key)),
  );
  if (Object.keys(constraints).length === 0) {
    return { $ref, ...annotations };
  }
  return { ...annotations, allOf: [{ $ref }, constraints] };
}

export async function generateWebTypes(outputDirectory = defaultOutput) {
  await mkdir(outputDirectory, { recursive: true });

  for (const schema of schemas) {
    const parsed = await readSchema(schema.source);
    const source = await compile(normalizeRefSiblings(parsed), parsed.title, {
      bannerComment,
      cwd: path.dirname(schema.source),
      enableConstEnums: false,
      unknownAny: true,
      unreachableDefinitions: true,
    });
    await writeFile(path.join(outputDirectory, schema.output), source, "utf8");
  }

  const clientSchema = await readSchema(schemas[0].source);
  const serverSchema = await readSchema(schemas[1].source);
  const clientVersion = protocolVersion(clientSchema, schemas[0].source);
  const serverVersion = protocolVersion(serverSchema, schemas[1].source);
  if (clientVersion !== serverVersion) {
    throw new Error("client and server schemas declare different protocol versions");
  }

  const metadata = `${bannerComment}
export const PROTOCOL_VERSION = ${clientVersion} as const;
export const CLIENT_MESSAGE_TYPES = [${quoteList(messageTypes(clientSchema, schemas[0].source))}] as const;
export const SERVER_MESSAGE_TYPES = [${quoteList(messageTypes(serverSchema, schemas[1].source))}] as const;
`;
  await writeFile(path.join(outputDirectory, "wire-metadata.ts"), metadata, "utf8");
  await writeFile(
    path.join(outputDirectory, "index.ts"),
    `${bannerComment}
export type { ClientMessage } from "./client-message";
export type { ServerMessage } from "./server-message";
export { CLIENT_MESSAGE_TYPES, PROTOCOL_VERSION, SERVER_MESSAGE_TYPES } from "./wire-metadata";
`,
    "utf8",
  );
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  await generateWebTypes(outputArgument(process.argv.slice(2)));
}
