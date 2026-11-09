import { afterEach, describe, expect, it } from "bun:test";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { compareVersions, readVersion, resolveTarget, writeManifest } from "./version";
import { prependChangelog, suggestBump } from "./release";

describe("compareVersions", () => {
  it("compares semver strings accurately", () => {
    expect(compareVersions("0.1.0", "0.1.0")).toBe(0);
    expect(compareVersions("0.2.0", "0.1.9")).toBe(1);
    expect(compareVersions("0.1.1", "0.1.2")).toBe(-1);
    expect(compareVersions("1.0.0", "0.9.9")).toBe(1);
  });
});

describe("resolveTarget", () => {
  it("handles explicit versions", () => {
    expect(resolveTarget("0.2.0", "0.1.0")).toBe("0.2.0");
    expect(resolveTarget("v1.0.0", "0.1.0")).toBe("1.0.0");
  });

  it("bumps patch, minor, major relative to base", () => {
    expect(resolveTarget("patch", "0.1.0")).toBe("0.1.1");
    expect(resolveTarget("minor", "0.1.5")).toBe("0.2.0");
    expect(resolveTarget("major", "0.2.3")).toBe("1.0.0");
  });
});
describe("writeManifest", () => {
  let dir = "";
  afterEach(() => {
    if (dir) rmSync(dir, { recursive: true, force: true });
  });
  const makeDir = () => {
    dir = mkdtempSync(join(tmpdir(), "alolite-manifest-test-"));
    return dir;
  };

  it("updates Cargo.toml workspace version cleanly", () => {
    const d = makeDir();
    const tomlPath = join(d, "Cargo.toml");
    writeFileSync(
      tomlPath,
      '[workspace.package]\nname = "alolite"\nversion = "0.1.0"\nedition = "2021"\n',
      "utf8",
    );
    writeManifest(tomlPath, "0.2.0");
    expect(readVersion(tomlPath)).toBe("0.2.0");
    expect(readFileSync(tomlPath, "utf8")).toContain('version = "0.2.0"');
  });
  it("updates package.json version cleanly", () => {
    const d = makeDir();
    const jsonPath = join(d, "package.json");
    writeFileSync(jsonPath, '{\n  "name": "alolite-desktop",\n  "version": "0.1.0"\n}\n', "utf8");
    writeManifest(jsonPath, "0.2.0");
    expect(readVersion(jsonPath)).toBe("0.2.0");
  });
});

describe("suggestBump", () => {
  it("suggests patch when chores/fixes merge", () => {
    const prs = [
      { number: 1, title: "fix(chat): bubble alignment", mergedAt: "", labels: [] },
      { number: 2, title: "chore: update deps", mergedAt: "", labels: [] },
    ];
    expect(suggestBump(prs)?.level).toBe("patch");
  });

  it("suggests minor when feat merges", () => {
    const prs = [{ number: 1, title: "feat(ui): add tools popover", mergedAt: "", labels: [] }];
    expect(suggestBump(prs)?.level).toBe("minor");
  });

  it("suggests major when breaking change merges", () => {
    const prs = [
      { number: 1, title: "feat(core)!: change protocol cipher", mergedAt: "", labels: [] },
    ];
    expect(suggestBump(prs)?.level).toBe("major");
  });
});

describe("prependChangelog", () => {
  const dir = mkdtempSync(join(tmpdir(), "alolite-changelog-test-"));
  const path = join(dir, "CHANGELOG.md");
  afterEach(() => {
    rmSync(dir, { recursive: true, force: true });
  });

  it("prepends new version under header", () => {
    prependChangelog("0.1.1", "- Initial patch release", path);
    const text = readFileSync(path, "utf8");
    expect(text).toContain("# Changelog");
    expect(text).toContain("## 0.1.1 - ");
    expect(text).toContain("- Initial patch release");
  });
});
