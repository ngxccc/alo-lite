#!/usr/bin/env bun
/**
 * Automated release pipeline: bump -> changelog -> tag -> GitHub release.
 * Adapted from open-genset/scripts/release.ts for Alolite's Cargo + Tauri workspace.
 *
 * Usage:
 *   bun scripts/release.ts                     # infer bump level from merged PRs
 *   bun scripts/release.ts 0.1.1               # explicit target version
 *   bun scripts/release.ts patch|minor|major   # bump relative to current/tag
 *   bun scripts/release.ts --dry-run           # preview proposed plan without changes
 */
import { readFileSync, writeFileSync } from "node:fs";
import {
  compareVersions,
  currentVersion,
  MANIFESTS,
  resolveTarget,
  setVersion,
} from "./version";

export const CHANGELOG = "CHANGELOG.md";

function fail(message: string): never {
  console.error(`\n✖ ${message}`);
  process.exit(1);
}

function step(message: string): void {
  console.log(`\n▸ ${message}`);
}

type RunOptions = { capture?: boolean; allowFailure?: boolean; input?: string };

function run(cmd: string[], options: RunOptions = {}): string {
  const { capture = false, allowFailure = false, input } = options;
  const proc = Bun.spawnSync({
    cmd,
    stdin: input ? new TextEncoder().encode(input) : "inherit",
    stdout: capture ? "pipe" : "inherit",
    stderr: capture ? "pipe" : "inherit",
  });
  const output = capture ? (proc.stdout?.toString() ?? "").trim() : "";
  if (proc.exitCode !== 0 && !allowFailure) {
    if (capture) console.error(proc.stderr?.toString().trim());
    fail(`command failed (${proc.exitCode}): ${cmd.join(" ")}`);
  }
  return output;
}

export function latestTag(): string | null {
  const tags = run(["git", "tag", "--list", "v*", "--sort=-v:refname"], {
    capture: true,
  })
    .split("\n")
    .filter(Boolean);
  return tags[0] ?? null;
}

export type MergedPullRequest = {
  number: number;
  title: string;
  mergedAt: string;
  mergeCommit?: { oid: string } | null;
  labels: { name: string }[];
};

export type Suggestion = { level: "major" | "minor" | "patch"; reason: string };

export function isBreaking(pr: MergedPullRequest): boolean {
  return (
    /^\w+(?:\([^)]*\))?!:/.test(pr.title) ||
    pr.labels.some((l) => l.name === "breaking-change" || l.name === "breaking")
  );
}

export function isFeature(pr: MergedPullRequest): boolean {
  return (
    /^feat(?:\([^)]*\))?!?:/i.test(pr.title) ||
    pr.labels.some((l) => l.name === "type:feat" || l.name === "feature")
  );
}

export function suggestBump(prs: MergedPullRequest[]): Suggestion | null {
  if (prs.length === 0) return null;
  const breaking = prs.find(isBreaking);
  if (breaking) {
    return { level: "major", reason: `#${breaking.number} is a breaking change` };
  }
  const feature = prs.find(isFeature);
  if (feature) {
    return { level: "minor", reason: `#${feature.number} adds a feature` };
  }
  return { level: "patch", reason: "no feature, no breaking change" };
}

export function seedChangelog(path: string = CHANGELOG): void {
  const header = [
    "# Changelog",
    "",
    "All notable changes to Alolite are documented in this file.",
    "Generated automatically from merged pull requests and conventional commits.",
    "",
  ].join("\n");

  try {
    writeFileSync(path, header, { flag: "wx" });
  } catch (error) {
    if ((error as { code?: string }).code !== "EEXIST") throw error;
  }
}

export function prependChangelog(
  target: string,
  body: string,
  path: string = CHANGELOG,
): void {
  seedChangelog(path);
  const date = new Date().toISOString().slice(0, 10);
  const heading = `## [${target}] - ${date}\n\n${body.trim()}\n`;
  const existing = readFileSync(path, "utf8");

  const preambleEnd = existing.indexOf("\n## ");
  const updated =
    preambleEnd === -1
      ? `${existing.trimEnd()}\n\n${heading}`
      : `${existing.slice(0, preambleEnd).trimEnd()}\n\n${heading}\n${existing.slice(preambleEnd + 1)}`;
  writeFileSync(path, updated, "utf8");
}

function repoSlug(): string {
  return run(
    ["gh", "repo", "view", "--json", "nameWithOwner", "--jq", ".nameWithOwner"],
    { capture: true },
  );
}

function generateNotes(target: string, head: string): string {
  const previous = latestTag();
  const fields = [
    "-f",
    `tag_name=v${target}`,
    "-f",
    `target_commitish=${head}`,
    ...(previous ? ["-f", `previous_tag_name=${previous}`] : []),
  ];
  return run(
    [
      "gh",
      "api",
      "--method",
      "POST",
      `repos/${repoSlug()}/releases/generate-notes`,
      ...fields,
      "--jq",
      ".body",
    ],
    { capture: true, allowFailure: true },
  );
}

if (import.meta.main) {
  const args = process.argv.slice(2);
  const dryRun = args.includes("--dry-run");
  const input = args.find((arg) => !arg.startsWith("--"));

  let target: string;

  if (input) {
    target = resolveTarget(input, latestTag()?.replace(/^v/, "") ?? currentVersion());
  } else {
    target = resolveTarget("patch", latestTag()?.replace(/^v/, "") ?? currentVersion());
  }

  const current = currentVersion() ?? "0.0.0";
  step(`Target release: v${target} (current: v${current})${dryRun ? " [DRY RUN]" : ""}`);

  if (compareVersions(target, current) <= 0) {
    fail(`v${target} must be greater than current v${current}`);
  }

  if (dryRun) {
    console.log(`
Plan:
  1. Synchronize version in manifests:
     ${MANIFESTS.join("\n     ")}
  2. Run cargo check to sync Cargo.lock
  3. Prepend release notes to ${CHANGELOG}
  4. Commit: chore(release): v${target}
  5. Create annotated git tag: v${target}
  6. Push commit and tag to origin
  7. GitHub Actions release.yml triggers to build packages (.deb, .rpm, .AppImage, .tar.gz)
`);
    process.exit(0);
  }

  step("Updating manifests & Cargo.lock");
  setVersion(target, true);

  step("Updating changelog");
  const headSha = run(["git", "rev-parse", "HEAD"], { capture: true });
  const notes = generateNotes(target, headSha) || `### Alolite v${target}\n\n- Release version ${target}.`;
  prependChangelog(target, notes);

  step("Staging and committing");
  run(["git", "add", ...MANIFESTS, "Cargo.lock", CHANGELOG]);
  run(["git", "commit", "-m", `chore(release): v${target}`]);
  run(["git", "tag", "-a", `v${target}`, "-m", `Alolite v${target}`]);

  console.log(`\n✔ Tagged v${target}. Push with: git push origin main --tags`);
}
