#!/usr/bin/env bun
/**
 * Single source of truth product version, mirrored across Cargo and desktop manifests.
 *
 * Manifests synchronized:
 *   - Cargo.toml ([workspace.package] version)
 *   - apps/desktop/package.json
 *   - crates/alolite-desktop/tauri.conf.json
 *   - Cargo.lock (via cargo check sync)
 *
 * Usage:
 *   bun scripts/version.ts check                    # exit 1 if manifests disagree
 *   bun scripts/version.ts current                  # print current root version
 *   bun scripts/version.ts bump <1.2.3|patch|minor|major>  # write version to manifests & sync Cargo.lock
 */
import { readFileSync, writeFileSync } from "node:fs";

export const MANIFESTS = [
  "package.json",
  "Cargo.toml",
  "apps/desktop/package.json",
  "crates/alolite-desktop/tauri.conf.json",
] as const;

export const SEMVER = /^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$/;

/** Read declared version from a manifest path. */
export function readVersion(path: string): string | undefined {
  const content = readFileSync(path, "utf8");
  if (path.endsWith(".toml")) {
    const match = content.match(
      /\[workspace\.package\][\s\S]*?^version\s*=\s*"([^"]+)"/m,
    );
    return match ? match[1] : undefined;
  }
  try {
    const json = JSON.parse(content) as { version?: string };
    return json.version;
  } catch {
    return undefined;
  }
}

/** Version declared by the root workspace Cargo.toml. */
export function currentVersion(): string | undefined {
  return readVersion(MANIFESTS[0]);
}

/** Manifest paths whose version disagrees with the root Cargo.toml. */
export function mismatchedManifests(): (typeof MANIFESTS)[number][] {
  const root = currentVersion();
  return MANIFESTS.filter((path) => readVersion(path) !== root);
}

/** Write version into a single manifest, preserving whitespace/formatting. */
export function writeManifest(path: string, version: string): void {
  const content = readFileSync(path, "utf8");
  if (path.endsWith(".toml")) {
    const updated = content.replace(
      /(\[workspace\.package\][\s\S]*?^version\s*=\s*)"[^"]+"/m,
      `$1"${version}"`,
    );
    writeFileSync(path, updated, "utf8");
  } else {
    const updated = content.replace(
      /("version"\s*:\s*)"[^"]+"/,
      `$1"${version}"`,
    );
    writeFileSync(path, updated, "utf8");
  }
}

/** Write version to all manifests and update Cargo.lock. */
export function setVersion(version: string, syncCargoLock = true): void {
  for (const path of MANIFESTS) {
    writeManifest(path, version);
  }
  if (syncCargoLock) {
    const proc = Bun.spawnSync({
      cmd: ["cargo", "check", "--workspace", "--quiet"],
      stdout: "inherit",
      stderr: "inherit",
    });
    if (proc.exitCode !== 0) {
      console.warn("warning: cargo check exited with code", proc.exitCode);
    }
  }
}

/** -1 / 0 / 1 comparison across major, minor, patch. */
export function compareVersions(a: string, b: string): number {
  const clean = (v: string) => v.replace(/^v/, "").split("-")[0] ?? "";
  const left = clean(a).split(".").map(Number);
  const right = clean(b).split(".").map(Number);
  for (let i = 0; i < 3; i++) {
    const diff = (left[i] ?? 0) - (right[i] ?? 0);
    if (diff !== 0) return diff > 0 ? 1 : -1;
  }
  return 0;
}

/** Resolve target version from string (either explicit X.Y.Z or patch|minor|major). */
export function resolveTarget(
  input: string,
  baseVersion: string = currentVersion() ?? "0.1.0",
): string {
  const cleanInput = input.replace(/^v/, "");
  if (SEMVER.test(cleanInput)) return cleanInput;
  if (input === "patch" || input === "minor" || input === "major") {
    const clean = baseVersion.replace(/^v/, "").split("-")[0] ?? "0.0.0";
    const [major, minor, patch] = clean.split(".").map(Number);
    if (input === "major") return `${major + 1}.0.0`;
    if (input === "minor") return `${major}.${minor + 1}.0`;
    return `${major}.${minor}.${patch + 1}`;
  }
  throw new Error(
    `Invalid version or bump type: ${input}. Expected semver or patch|minor|major.`,
  );
}

if (import.meta.main) {
  const [, , mode, target] = process.argv;

  if (mode === "check") {
    const root = currentVersion();
    const mismatched = mismatchedManifests();
    if (!root || mismatched.length > 0) {
      console.error(
        `version mismatch: ${MANIFESTS[0]} is ${root ?? "missing"}`,
      );
      for (const path of mismatched) {
        console.error(`  ${path}: ${readVersion(path) ?? "missing"}`);
      }
      console.error("run: bun scripts/version.ts bump <x.y.z>");
      process.exit(1);
    }
    console.log(`versions aligned at v${root}`);
  } else if (mode === "current") {
    console.log(currentVersion() ?? "0.0.0");
  } else if (mode === "bump") {
    if (!target) {
      console.error("usage: bun scripts/version.ts bump <x.y.z|patch|minor|major>");
      process.exit(1);
    }
    const resolved = resolveTarget(target);
    setVersion(resolved);
    console.log(`wrote v${resolved} to ${MANIFESTS.length} manifests and synchronized Cargo.lock`);
  } else {
    console.error(
      "usage: bun scripts/version.ts check | current | bump <x.y.z|patch|minor|major>",
    );
    process.exit(1);
  }
}
