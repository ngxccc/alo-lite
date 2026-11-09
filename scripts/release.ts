#!/usr/bin/env bun
/**
 * Cut a release in one command: bump → changelog → PR → merge → tag → publish.
 * Adapted directly from open-genset/scripts/release.ts for Alolite.
 *
 * `main` is protected, so the bump cannot be pushed directly; it travels as a PR that
 * contains only the version numbers and the new CHANGELOG.md section. The script waits for
 * that PR's checks, squash-merges it, then creates the tag and the GitHub release.
 *
 * Notes come from GitHub's own release-notes generator (`generate-notes`, the same one behind
 * `gh release create --generate-notes`), so `.github/release.yml` stays the only place that
 * decides categories. We call it once and write the result into both CHANGELOG.md and the
 * release page, which is why the two can never disagree.
 *
 * Usage:
 *   bun run release                  # infer the level from merged pull requests
 *   bun run release 1.2.3            # explicit version
 *   bun run release minor            # patch | minor | major, from the latest tag
 *   bun run release --dry-run        # validate and print the plan, change nothing
 */
import { readFileSync, writeFileSync } from "node:fs";
import { compareVersions, currentVersion, MANIFESTS, resolveTarget, setVersion } from "./version";
export { resolveTarget } from "./version";
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
    pr.labels.some((label) => label.name === "breaking-change")
  );
}

export function isFeature(pr: MergedPullRequest): boolean {
  return (
    /^feat(?:\([^)]*\))?!?:/i.test(pr.title) ||
    pr.labels.some((label) => label.name === "type:feat")
  );
}

/**
 * The level a set of merged pull requests justifies, derived from their titles
 * and the `type:*` labels the notes group by.
 */
export function suggestBump(prs: MergedPullRequest[]): Suggestion | null {
  if (prs.length === 0) return null;

  const breaking = prs.find(isBreaking);
  if (breaking) {
    return {
      level: "major",
      reason: `#${breaking.number} is a breaking change`,
    };
  }

  const feature = prs.find(isFeature);
  if (feature) {
    return { level: "minor", reason: `#${feature.number} adds a feature` };
  }

  return { level: "patch", reason: "no feature, no breaking change" };
}

/** Pull requests merged into `main` after the previous tag was cut. */
export function mergedSince(tag: string | null): MergedPullRequest[] {
  const since = tag
    ? new Date(run(["git", "log", "-1", "--format=%cI", tag], { capture: true })).getTime()
    : 0;
  const raw = run(
    [
      "gh",
      "pr",
      "list",
      "--state",
      "merged",
      "--base",
      "main",
      "--limit",
      "500",
      "--json",
      "number,title,mergedAt,mergeCommit,labels",
    ],
    { capture: true },
  );
  let prs: MergedPullRequest[] = [];
  try {
    prs = JSON.parse(raw) as MergedPullRequest[];
  } catch {
    prs = [];
  }

  return prs.filter((pr) => {
    if (new Date(pr.mergedAt).getTime() <= since) return false;

    if (pr.mergeCommit?.oid && tag) {
      const contained = Bun.spawnSync({
        cmd: ["git", "merge-base", "--is-ancestor", pr.mergeCommit.oid, tag],
        stdout: "ignore",
        stderr: "ignore",
      });
      if (contained.exitCode === 0) return false;
    }

    return true;
  });
}

function assertReady(target: string): void {
  if (run(["git", "status", "--porcelain"], { capture: true }) !== "") {
    fail("working tree is dirty — commit or stash before releasing");
  }

  const branch = run(["git", "rev-parse", "--abbrev-ref", "HEAD"], {
    capture: true,
  });
  if (branch !== "main") {
    fail(`release from main, not from ${branch}`);
  }

  run(["git", "fetch", "origin", "main", "--quiet"]);
  const head = run(["git", "rev-parse", "HEAD"], { capture: true });
  const remote = run(["git", "rev-parse", "origin/main"], { capture: true });
  if (head !== remote) {
    fail("local main is not origin/main — pull first");
  }

  const latest = latestTag()?.replace(/^v/, "");
  const reference = latest ?? currentVersion();
  if (reference && compareVersions(target, reference) <= 0) {
    fail(`v${target} is not newer than v${reference}`);
  }

  if (run(["git", "tag", "--list", `v${target}`], { capture: true })) {
    fail(`tag v${target} already exists`);
  }
  const existing = Bun.spawnSync({
    cmd: ["gh", "release", "view", `v${target}`],
    stdout: "ignore",
    stderr: "ignore",
  });
  if (existing.exitCode === 0) {
    fail(`release v${target} already exists`);
  }
}

function repoSlug(): string {
  const remote = run(["git", "remote", "get-url", "origin"], {
    capture: true,
    allowFailure: true,
  });
  const match = remote.match(/[:/]([^/]+\/[^/]+?)(?:\.git)?$/);
  if (match) return match[1];
  return run(["gh", "repo", "view", "--json", "nameWithOwner", "--jq", ".nameWithOwner"], {
    capture: true,
  });
}

/** Notes for the release we are about to cut, from GitHub's generator. */
export function generateNotes(target: string, head: string): string {
  console.log("  Connecting to GitHub API to compute release notes...");
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
    { capture: true },
  );
}

export function seedChangelog(path: string = CHANGELOG): void {
  const header = [
    "# Changelog",
    "",
    "Generated at release time from merged pull requests, by `bun run release`.",
    "Do not edit by hand; the sections come from the same generator as the GitHub release.",
    "",
  ].join("\n");

  const previous = latestTag();
  if (!previous) {
    createOnly(path, header);
    return;
  }

  const body = run(["gh", "release", "view", previous, "--json", "body", "--jq", ".body"], {
    capture: true,
    allowFailure: true,
  });
  const date = run(["git", "log", "-1", "--format=%ad", "--date=short", previous], {
    capture: true,
    allowFailure: true,
  });
  createOnly(path, `${header}\n## ${previous.replace(/^v/, "")} - ${date}\n\n${body}\n`);
}

/** Create the file only if it does not exist, without a check-then-write race. */
function createOnly(path: string, content: string): void {
  try {
    writeFileSync(path, content, { flag: "wx" });
  } catch (error) {
    if ((error as { code?: string }).code !== "EEXIST") throw error;
  }
}

/** Put the newest version directly under the preamble. */
export function prependChangelog(target: string, body: string, path: string = CHANGELOG): void {
  console.log(`  Writing entry for v${target} to ${path}...`);
  seedChangelog(path);
  const date = new Date().toISOString().slice(0, 10);
  const heading = `## ${target} - ${date}\n\n${body}\n`;
  const existing = readFileSync(path, "utf8");

  const preambleEnd = existing.indexOf("\n## ");
  const updated =
    preambleEnd === -1
      ? `${existing.trimEnd()}\n\n${heading}`
      : `${existing.slice(0, preambleEnd).trimEnd()}\n\n${heading}${existing.slice(preambleEnd + 1)}`;
  writeFileSync(path, updated);
}

if (import.meta.main) {
  const args = process.argv.slice(2);
  const dryRun = args.includes("--dry-run");
  const input = args.find((arg) => !arg.startsWith("--"));

  let target: string;

  if (input) {
    target = resolveTarget(input, latestTag()?.replace(/^v/, "") ?? currentVersion());
  } else {
    const previous = latestTag();
    const merged = mergedSince(previous);
    const suggestion = suggestBump(merged);
    if (!suggestion) {
      fail(`nothing merged since ${previous ?? "the first release"} — nothing to release`);
    }

    target = resolveTarget(suggestion.level, previous?.replace(/^v/, "") ?? currentVersion());
    const count = `${merged.length} pull request${merged.length === 1 ? "" : "s"}`;
    step(`No version given — ${count} merged since ${previous ?? "the beginning"}`);
    for (const pr of merged.slice(0, 10)) {
      console.log(`  #${pr.number}  ${pr.title}`);
    }
    if (merged.length > 10) console.log(`  … and ${merged.length - 10} more`);
    console.log(`  → v${target} (${suggestion.level}: ${suggestion.reason})`);

    if (!dryRun) {
      if (!process.stdin.isTTY) {
        fail(`no terminal to confirm on — rerun as: bun run release ${target}`);
      }
      const answer = prompt(`\nRelease v${target}? [y/N]`);
      if (!/^y(es)?$/i.test((answer ?? "").trim())) {
        console.log("\nAborted — nothing changed.");
        process.exit(0);
      }
    }
  }

  const branch = `chore/release-v${target}`;
  const head = run(["git", "rev-parse", "HEAD"], { capture: true });

  step(`Release v${target}${dryRun ? " (dry run)" : ""}`);
  assertReady(target);
  console.log(`  from ${head.slice(0, 7)} on main, previous tag ${latestTag() ?? "none"}`);

  if (dryRun) {
    console.log(`
  Plan:
    1. branch ${branch}
    2. generate notes, prepend ${CHANGELOG}
    3. write ${target} into ${MANIFESTS.length} manifests and sync Cargo.lock
    4. commit chore(release): v${target} and push
    5. open PR against main for review
    6. return local branch to main
    (Once merged, GitHub Actions automatically tags and releases v${target})
  `);
    process.exit(0);
  }

  step(`Branch ${branch}`);
  run(["git", "switch", "-c", branch]);

  step("Generate release notes");
  const notes = generateNotes(target, head);
  if (!notes.trim()) fail("release notes came back empty");
  prependChangelog(target, notes);

  step(`Write ${target} into ${MANIFESTS.length} manifests`);
  setVersion(target);

  step(`Commit and push`);
  run(["git", "add", ...MANIFESTS, "Cargo.lock", CHANGELOG]);
  run(["git", "commit", "-m", `chore(release): v${target}`]);
  run(["git", "push", "-u", "origin", branch]);

  step("Open pull request");
  const url = run(
    [
      "gh",
      "pr",
      "create",
      "--base",
      "main",
      "--head",
      branch,
      "--title",
      `chore(release): v${target}`,
      "--body",
      `Version bump to v${target} plus the ${CHANGELOG} entry for it.\n\nNotes are generated from merged PRs, so this PR is bookkeeping only.`,
    ],
    { capture: true },
  );
  const prNumber = Number(url.match(/\/pull\/(\d+)/)?.[1]);
  if (!prNumber) fail(`could not read PR number from: ${url}`);
  console.log(`  ${url}`);

  step("Return to main");
  run(["git", "switch", "main"]);

  console.log(`\n✔ Release PR #${prNumber} created: ${url}`);
  console.log("  Review the changes and CHANGELOG.md on GitHub, then squash-merge the PR.");
  console.log(
    `  Once merged into main, GitHub Actions will automatically tag and release v${target}!\n`,
  );
}
