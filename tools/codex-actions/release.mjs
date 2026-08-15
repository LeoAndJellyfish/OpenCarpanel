#!/usr/bin/env node

import { spawn } from "node:child_process";
import path from "node:path";
import process from "node:process";
import { createInterface } from "node:readline/promises";
import { fileURLToPath } from "node:url";

const scriptPath = fileURLToPath(import.meta.url);
const projectRoot = path.resolve(path.dirname(scriptPath), "..", "..");
const semverPattern =
  /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-((?:0|[1-9]\d*|\d*[A-Za-z-][0-9A-Za-z-]*)(?:\.(?:0|[1-9]\d*|\d*[A-Za-z-][0-9A-Za-z-]*))*))?(?:\+([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?$/;

export function normalizeVersion(input) {
  if (typeof input !== "string") {
    throw new TypeError("版本号必须是字符串。");
  }

  const candidate = input.trim().replace(/^v/i, "");
  if (!semverPattern.test(candidate)) {
    throw new Error(
      "版本号必须是完整 SemVer，例如 0.3.4、v0.4.0 或 1.0.0-rc.1。",
    );
  }

  return { tag: `v${candidate}`, version: candidate };
}

export function buildReleasePrompt(tag) {
  return [
    `为 OpenSimDash 发布 ${tag}。`,
    "使用 $github-issue-to-release skill 执行完整、可审计的发布闭环。",
    "先检查工作区、origin/main、当前最新版本、目标 tag 与 Release 是否已存在；不要覆盖已有 tag。",
    "根据目标版本完成必要的版本文件、发布说明和工作流文案更新，保留用户已有改动。",
    "运行与风险相称的完整本地门禁；通过后提交并推送 main。",
    "只接受精确提交 SHA 的 main CI 成功结果，然后创建并推送带注释 tag。",
    "等待 Release 工作流完成，并独立核验 job、资产矩阵、latest.json、签名和公开下载链接。",
    "只有所有验收均通过后才关闭相关 Issue；若存在真实设备或签名边界，在最终结果中明确说明。",
  ].join("\n");
}

function printUsage() {
  console.log("用法：node tools/codex-actions/release.mjs [--dry-run] [版本号]");
  console.log("未提供版本号时会在终端中交互询问。");
}

function parseArguments(args) {
  let dryRun = false;
  let versionInput = null;

  for (const argument of args) {
    if (argument === "--dry-run") {
      dryRun = true;
      continue;
    }
    if (argument === "--help" || argument === "-h") {
      return { help: true };
    }
    if (versionInput !== null) {
      throw new Error(`无法识别的额外参数：${argument}`);
    }
    versionInput = argument;
  }

  return { dryRun, help: false, versionInput };
}

async function launchCodex(prompt) {
  const executable = process.platform === "win32" ? "codex.exe" : "codex";
  const child = spawn(executable, ["--cd", projectRoot, prompt], {
    cwd: projectRoot,
    stdio: "inherit",
    windowsHide: false,
  });

  const result = await new Promise((resolve, reject) => {
    child.once("error", reject);
    child.once("exit", (code, signal) => resolve({ code, signal }));
  });

  if (result.signal !== null) {
    throw new Error(`Codex 发布会话被信号 ${result.signal} 中止。`);
  }
  if (result.code !== 0) {
    throw new Error(`Codex 发布会话退出码为 ${result.code ?? "unknown"}。`);
  }
}

export async function main(args = process.argv.slice(2)) {
  const options = parseArguments(args);
  if (options.help) {
    printUsage();
    return;
  }

  const terminal = createInterface({ input: process.stdin, output: process.stdout });
  let terminalClosed = false;
  try {
    const rawVersion =
      options.versionInput ?? (await terminal.question("请输入要发布的版本号："));
    const { tag } = normalizeVersion(rawVersion);
    const prompt = buildReleasePrompt(tag);

    console.log(`\n目标版本：${tag}`);
    if (options.dryRun) {
      console.log("\n将交给 Codex 的任务：\n");
      console.log(prompt);
      return;
    }

    console.log(
      "该操作会启动一个新的交互式 Codex CLI 会话；发布会话仍会检查、测试并在外部写入前执行必要确认。",
    );
    const confirmation = await terminal.question(
      "输入 release 继续，其他任意内容取消：",
    );
    if (confirmation.trim().toLowerCase() !== "release") {
      console.log("已取消发布。仓库未被此脚本修改。");
      return;
    }

    terminal.close();
    terminalClosed = true;
    await launchCodex(prompt);
  } finally {
    if (!terminalClosed) {
      terminal.close();
    }
  }
}

const invokedPath = process.argv[1] ? path.resolve(process.argv[1]) : null;
const normalizedInvokedPath =
  process.platform === "win32" ? invokedPath?.toLowerCase() : invokedPath;
const normalizedScriptPath =
  process.platform === "win32" ? scriptPath.toLowerCase() : scriptPath;

if (normalizedInvokedPath === normalizedScriptPath) {
  await main();
}
