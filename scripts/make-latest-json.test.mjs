// Tests for make-latest-json.mjs. Run with:
//   node scripts/make-latest-json.test.mjs
// (uses node:test; no vitest involvement on purpose)

import { test } from "node:test";
import assert from "node:assert/strict";
import { mkdtempSync, writeFileSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import {
  versionFromTag,
  assertTagMatchesVersion,
  assetUrl,
  buildLatestJson,
  parseArgs,
  main,
} from "./make-latest-json.mjs";

test("versionFromTag extracts X.Y.Z", () => {
  assert.equal(versionFromTag("v0.1.1"), "0.1.1");
  assert.equal(versionFromTag("v1.2.3-beta.1"), "1.2.3-beta.1");
  assert.throws(() => versionFromTag("0.1.1"), /does not look like a release tag/);
  assert.throws(() => versionFromTag("v1.2"), /does not look like a release tag/);
});

test("assertTagMatchesVersion accepts a match, rejects a mismatch", () => {
  assert.equal(assertTagMatchesVersion("v0.1.1", "0.1.1"), "0.1.1");
  assert.throws(
    () => assertTagMatchesVersion("v0.1.2", "0.1.1"),
    /Version mismatch: tag "v0\.1\.2" says 0\.1\.2 but src-tauri\/tauri\.conf\.json says 0\.1\.1/,
  );
});

test("assetUrl encodes spaces in the asset name", () => {
  assert.equal(
    assetUrl("v0.1.1", "Lore Desktop_0.1.1_x64-setup.exe"),
    "https://github.com/KiraFR/lore-desktop/releases/download/v0.1.1/Lore%20Desktop_0.1.1_x64-setup.exe",
  );
});

test("buildLatestJson produces the Tauri v2 updater shape", () => {
  const manifest = buildLatestJson({
    version: "0.1.1",
    notes: "notes",
    pubDate: "2026-07-22T00:00:00.000Z",
    signature: "SIG",
    url: "https://example.com/a.exe",
  });
  assert.deepEqual(manifest, {
    version: "0.1.1",
    notes: "notes",
    pub_date: "2026-07-22T00:00:00.000Z",
    platforms: {
      "windows-x86_64": { signature: "SIG", url: "https://example.com/a.exe" },
    },
  });
});

test("parseArgs handles positionals and flags", () => {
  const args = parseArgs(["a.exe", "a.exe.sig", "v1.0.0", "notes", "--out", "o.json"]);
  assert.equal(args.setupPath, "a.exe");
  assert.equal(args.sigPath, "a.exe.sig");
  assert.equal(args.tag, "v1.0.0");
  assert.equal(args.notes, "notes");
  assert.equal(args.out, "o.json");
  assert.throws(() => parseArgs(["a.exe"]), /Usage:/);
  assert.throws(() => parseArgs(["a", "b", "v1.0.0", "--bogus"]), /Unknown option/);
});

test("main end-to-end with fake artifacts", () => {
  const dir = mkdtempSync(join(tmpdir(), "make-latest-json-"));
  try {
    const setup = join(dir, "Lore Desktop_0.1.1_x64-setup.exe");
    const sig = `${setup}.sig`;
    const config = join(dir, "tauri.conf.json");
    const out = join(dir, "latest.json");
    writeFileSync(setup, "fake installer bytes");
    writeFileSync(sig, "dGVzdC1zaWduYXR1cmU=\n");
    writeFileSync(config, JSON.stringify({ productName: "Lore Desktop", version: "0.1.1" }));

    const manifest = main(
      [setup, sig, "v0.1.1", "--config", config, "--out", out],
      { now: () => new Date("2026-07-22T12:00:00Z"), log: () => {} },
    );

    assert.deepEqual(JSON.parse(readFileSync(out, "utf8")), manifest);
    assert.deepEqual(manifest, {
      version: "0.1.1",
      notes: "Lore Desktop v0.1.1",
      pub_date: "2026-07-22T12:00:00.000Z",
      platforms: {
        "windows-x86_64": {
          signature: "dGVzdC1zaWduYXR1cmU=",
          url: "https://github.com/KiraFR/lore-desktop/releases/download/v0.1.1/Lore%20Desktop_0.1.1_x64-setup.exe",
        },
      },
    });

    // --notes-file overrides the default notes
    const notesFile = join(dir, "notes.md");
    writeFileSync(notesFile, "Release notes body\n");
    const withNotes = main(
      [setup, sig, "v0.1.1", "--config", config, "--out", out, "--notes-file", notesFile],
      { now: () => new Date("2026-07-22T12:00:00Z"), log: () => {} },
    );
    assert.equal(withNotes.notes, "Release notes body");

    // guard rails
    assert.throws(
      () => main([setup, sig, "v9.9.9", "--config", config, "--out", out], { log: () => {} }),
      /Version mismatch/,
    );
    assert.throws(
      () => main([setup, join(dir, "missing.sig"), "v0.1.1", "--config", config, "--out", out], { log: () => {} }),
      /Signature file not found[\s\S]*signing secrets/,
    );
    assert.throws(
      () => main([join(dir, "missing.exe"), sig, "v0.1.1", "--config", config, "--out", out], { log: () => {} }),
      /Setup installer not found/,
    );
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});
