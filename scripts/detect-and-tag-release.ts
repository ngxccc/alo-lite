#!/usr/bin/env bun
/**
 * scripts/detect-and-tag-release.ts
 *
 * Detects release commits merged into main and creates/pushes git tags.
 * Outputs `should_release` and `tag_name` to $GITHUB_OUTPUT.
 */

import { spawnSync } from "node:child_process";
import { appendFileSync } from "node:fs";

const refType = process.argv[2] ?? process.env.GITHUB_REF_TYPE ?? "";
const refName = process.argv[3] ?? process.env.GITHUB_REF_NAME ?? "";
const eventName = process.argv[4] ?? process.env.GITHUB_EVENT_NAME ?? "";
const manualTag = process.argv[5] ?? "";
const outputFile = process.env.GITHUB_OUTPUT ?? "/dev/null";

function setOutput(shouldRelease: boolean, tagName = "") {
  appendFileSync(outputFile, `should_release=${shouldRelease}\ntag_name=${tagName}\n`, "utf8");
}

if (refType === "tag") {
  console.log(`Direct tag push detected: ${refName}`);
  setOutput(true, refName);
  process.exit(0);
}

if (eventName === "workflow_dispatch" && manualTag) {
  console.log(`Manual workflow dispatch release: ${manualTag}`);
  setOutput(true, manualTag);
  process.exit(0);
}

const logRes = spawnSync("git", ["log", "-1", "--pretty=%B"], { encoding: "utf-8" });
const commitMsg = logRes.stdout.trim();
console.log(`Evaluating recent commit: ${commitMsg}`);

const match = commitMsg.match(/chore\(release\):\s*v?([0-9]+\.[0-9]+\.[0-9]+)/);
if (match) {
  const tag = `v${match[1]}`;
  console.log(`Matched release commit for tag: ${tag}`);
  spawnSync("git", ["fetch", "--tags", "origin"]);

  const tagCheck = spawnSync("git", ["rev-parse", tag]);
  if (tagCheck.status === 0) {
    console.log(`Tag ${tag} already exists, skipping release.`);
    setOutput(false);
  } else {
    spawnSync("git", ["config", "user.name", "github-actions[bot]"]);
    spawnSync("git", ["config", "user.email", "github-actions[bot]@users.noreply.github.com"]);
    spawnSync("git", ["tag", "-a", tag, "-m", tag]);
    spawnSync("git", ["push", "origin", tag]);
    console.log(`Created and pushed tag ${tag}`);
    setOutput(true, tag);
  }
} else {
  console.log("Not a release commit, skipping release workflow.");
  setOutput(false);
}
