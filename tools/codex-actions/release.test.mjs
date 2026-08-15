import assert from "node:assert/strict";
import test from "node:test";

import { buildReleasePrompt, normalizeVersion } from "./release.mjs";

test("normalizes stable and prerelease SemVer tags", () => {
  assert.deepEqual(normalizeVersion("0.4.0"), {
    tag: "v0.4.0",
    version: "0.4.0",
  });
  assert.deepEqual(normalizeVersion(" v1.0.0-rc.1+build.5 "), {
    tag: "v1.0.0-rc.1+build.5",
    version: "1.0.0-rc.1+build.5",
  });
});

test("rejects incomplete, leading-zero, and shell-like versions", () => {
  for (const value of ["1.2", "v01.2.3", "0.4.0; git tag bad", "latest"]) {
    assert.throws(() => normalizeVersion(value));
  }
});

test("builds a bounded release task for the selected tag", () => {
  const prompt = buildReleasePrompt("v0.4.0");
  assert.match(prompt, /OpenCarpanel 发布 v0\.4\.0/);
  assert.match(prompt, /\$github-issue-to-release skill/);
  assert.match(prompt, /精确提交 SHA/);
  assert.match(prompt, /latest\.json/);
  assert.doesNotMatch(prompt, /v0\.3\.3/);
});
