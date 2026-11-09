#!/usr/bin/env bun
/**
 * scripts/pin-actions.ts
 *
 * Scans all GitHub Actions workflow and action YAML files,
 * resolves exact commit SHAs via git ls-remote,
 * and updates them to immutable commit hashes with version comments.
 *
 * Usage:
 *   bun scripts/pin-actions.ts           # In-place update
 *   bun scripts/pin-actions.ts --dry-run # Preview only
 */

import { readdir, readFile, writeFile } from "node:fs/promises";
import { join, resolve } from "node:path";
import { spawnSync } from "node:child_process";

const DRY_RUN = process.argv.includes("--dry-run");
const ROOT_DIR = resolve(import.meta.dir, "..");
const GITHUB_DIR = join(ROOT_DIR, ".github");

interface ResolvedAction {
  sha: string;
  tag: string;
}

const shaCache = new Map<string, ResolvedAction>();

function resolveActionSha(ownerRepo: string, ref: string): ResolvedAction | null {
  const cacheKey = `${ownerRepo}@${ref}`;
  if (shaCache.has(cacheKey)) {
    return shaCache.get(cacheKey)!;
  }

  // Already a 40-character hex commit SHA
  if (/^[0-9a-f]{40}$/i.test(ref)) {
    const res = { sha: ref, tag: ref };
    shaCache.set(cacheKey, res);
    return res;
  }

  // Split owner/repo from subpaths (e.g. github/codeql-action/init -> github/codeql-action)
  const parts = ownerRepo.split("/");
  const gitRepo = `${parts[0]}/${parts[1]}`;
  const repoUrl = `https://github.com/${gitRepo}.git`;

  // 1. Try direct tag, peeled tag (^{}), or branch head
  const lsRemote = spawnSync(
    "git",
    ["ls-remote", repoUrl, `refs/tags/${ref}^{}`, `refs/tags/${ref}`, `refs/heads/${ref}`],
    { encoding: "utf-8" },
  );

  if (lsRemote.status === 0 && lsRemote.stdout.trim()) {
    const lines = lsRemote.stdout.trim().split("\n");
    // Pick the last line (peeled tag ^{} comes last)
    const lastLine = lines[lines.length - 1];
    const sha = lastLine.split(/\s+/)[0];
    if (sha && sha.length === 40) {
      const res = { sha, tag: ref };
      shaCache.set(cacheKey, res);
      return res;
    }
  }

  // 2. Fallback: if tag does not exist (e.g. actions/cache@v7), query available semver tags
  const tagsQuery = spawnSync("git", ["ls-remote", "--tags", repoUrl, "refs/tags/v*"], {
    encoding: "utf-8",
  });

  if (tagsQuery.status === 0 && tagsQuery.stdout.trim()) {
    const tagLines = tagsQuery.stdout
      .trim()
      .split("\n")
      .filter((l) => !l.includes("^{}"));
    if (tagLines.length > 0) {
      // Sort semver tags
      tagLines.sort((a, b) => {
        const tagA = a.split(/\s+/)[1].replace("refs/tags/", "");
        const tagB = b.split(/\s+/)[1].replace("refs/tags/", "");
        return tagA.localeCompare(tagB, undefined, {
          numeric: true,
          sensitivity: "base",
        });
      });
      const latestLine = tagLines[tagLines.length - 1];
      const fallbackSha = latestLine.split(/\s+/)[0];
      const fallbackTag = latestLine.split(/\s+/)[1].replace("refs/tags/", "");
      console.warn(
        `⚠️  Warning: ${ownerRepo}@${ref} not found on remote. Falling back to latest tag: ${fallbackTag}`,
      );
      const res = {
        sha: fallbackSha,
        tag: `${fallbackTag} (fallback from ${ref})`,
      };
      shaCache.set(cacheKey, res);
      return res;
    }
  }

  console.error(`❌ Error: Could not resolve SHA for ${ownerRepo}@${ref}`);
  return null;
}

async function getYamlFiles(dir: string): Promise<string[]> {
  const files: string[] = [];
  try {
    const entries = await readdir(dir, { withFileTypes: true });
    for (const entry of entries) {
      const fullPath = join(dir, entry.name);
      if (entry.isDirectory()) {
        files.push(...(await getYamlFiles(fullPath)));
      } else if (entry.isFile() && (entry.name.endsWith(".yml") || entry.name.endsWith(".yaml"))) {
        files.push(fullPath);
      }
    }
  } catch {
    // Ignore missing directories
  }
  return files;
}

async function main() {
  console.log(`=== [1/2] Scanning GitHub workflows and composite actions ===\n`);
  if (DRY_RUN) {
    console.log(`ℹ️  Running in DRY-RUN mode: no files will be overwritten.\n`);
  }

  const yamlFiles = await getYamlFiles(GITHUB_DIR);
  let updatedFilesCount = 0;
  let totalReplacedActions = 0;

  for (const filePath of yamlFiles) {
    const relativePath = filePath.replace(`${ROOT_DIR}/`, "");
    const content = await readFile(filePath, "utf-8");
    const lines = content.split("\n");
    let fileModified = false;

    const newLines = lines.map((line) => {
      // Match `uses: owner/repo[/subpath]@ref`
      // Skip local actions starting with `./`
      const match = line.match(
        /^(\s*uses:\s*)([a-zA-Z0-9_-]+\/[a-zA-Z0-9_.-]+(?:\/[a-zA-Z0-9_.-]+)*)@([^\s#]+)(.*)$/,
      );
      if (!match) return line;

      const [, prefix, actionPath, ref] = match;
      if (actionPath.startsWith("./")) return line;

      // Skip if already a 40-char SHA
      if (/^[0-9a-f]{40}$/i.test(ref)) {
        return line;
      }

      const resolved = resolveActionSha(actionPath, ref);
      if (!resolved) {
        return line;
      }

      fileModified = true;
      totalReplacedActions++;
      return `${prefix}${actionPath}@${resolved.sha} # ${resolved.tag}`;
    });

    if (fileModified) {
      updatedFilesCount++;
      console.log(`  ✓ Updated: ${relativePath}`);
      if (!DRY_RUN) {
        await writeFile(filePath, newLines.join("\n"), "utf-8");
      }
    }
  }

  console.log(`\n=== [2/2] Action Pinning Resolution Summary ===\n`);
  console.log(`| ${"Action".padEnd(42)} | ${"Commit SHA".padEnd(42)} | Version Comment`);
  console.log(`| ${"-".repeat(42)} | ${"-".repeat(42)} | ${"-".repeat(20)}`);

  for (const [key, val] of shaCache.entries()) {
    console.log(`| ${key.padEnd(42)} | ${val.sha.padEnd(42)} | # ${val.tag}`);
  }

  console.log(
    `\nTotal files modified: ${updatedFilesCount} | Total actions pinned: ${totalReplacedActions}`,
  );
}

main().catch((err) => {
  console.error("Fatal error:", err);
  process.exit(1);
});
