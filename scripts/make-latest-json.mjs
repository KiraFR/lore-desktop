#!/usr/bin/env node
// Generates the Tauri v2 updater manifest (latest.json) for a Windows release.
//
// Usage:
//   node scripts/make-latest-json.mjs <setup.exe> <setup.exe.sig> <tag> [notes]
//     --notes-file <path>  read release notes from a file (overrides [notes])
//     --config <path>      tauri.conf.json path (default: src-tauri/tauri.conf.json)
//     --out <path>         output path (default: ./latest.json)
//
// <setup.exe>      path to the NSIS installer produced by `tauri build`
// <setup.exe.sig>  its minisign signature (base64), produced when
//                  bundle.createUpdaterArtifacts is enabled
// <tag>            the git tag being released, e.g. v0.1.1
//
// Pure Node, no dependencies. Exported functions are unit-tested by
// scripts/make-latest-json.test.mjs (run with: node --test scripts/).

import { readFileSync, writeFileSync, existsSync } from "node:fs";
import { basename, dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

export const REPO_SLUG = "KiraFR/lore-desktop";

const DEFAULT_CONFIG_PATH = join(
  dirname(fileURLToPath(import.meta.url)),
  "..",
  "src-tauri",
  "tauri.conf.json",
);

/** Extracts "X.Y.Z" from a "vX.Y.Z" tag. Throws if the tag is malformed. */
export function versionFromTag(tag) {
  const m = /^v(\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?)$/.exec(tag);
  if (!m) {
    throw new Error(
      `Tag "${tag}" does not look like a release tag (expected vX.Y.Z).`,
    );
  }
  return m[1];
}

/** Fails loudly when the tag and tauri.conf.json disagree on the version. */
export function assertTagMatchesVersion(tag, configVersion) {
  const tagVersion = versionFromTag(tag);
  if (tagVersion !== configVersion) {
    throw new Error(
      `Version mismatch: tag "${tag}" says ${tagVersion} but ` +
        `src-tauri/tauri.conf.json says ${configVersion}. ` +
        `Bump the version in tauri.conf.json before tagging.`,
    );
  }
  return tagVersion;
}

/**
 * Download URL of a release asset on this repo. Each path segment is
 * URL-encoded (the NSIS setup name contains spaces: "Lore Desktop_X.Y.Z_x64-setup.exe").
 */
export function assetUrl(tag, assetName, repoSlug = REPO_SLUG) {
  return (
    `https://github.com/${repoSlug}/releases/download/` +
    `${encodeURIComponent(tag)}/${encodeURIComponent(assetName)}`
  );
}

/** Assembles the Tauri v2 updater manifest object. */
export function buildLatestJson({ version, notes, pubDate, signature, url }) {
  return {
    version,
    notes,
    pub_date: pubDate,
    platforms: {
      "windows-x86_64": {
        signature,
        url,
      },
    },
  };
}

/** Minimal argv parser: positionals + the three known --flags. */
export function parseArgs(argv) {
  const flags = { config: DEFAULT_CONFIG_PATH, out: "latest.json", notesFile: null };
  const positionals = [];
  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i];
    if (arg === "--config" || arg === "--out" || arg === "--notes-file") {
      const value = argv[++i];
      if (value === undefined) throw new Error(`Missing value for ${arg}`);
      if (arg === "--config") flags.config = value;
      else if (arg === "--out") flags.out = value;
      else flags.notesFile = value;
    } else if (arg.startsWith("--")) {
      throw new Error(`Unknown option ${arg}`);
    } else {
      positionals.push(arg);
    }
  }
  const [setupPath, sigPath, tag, notes] = positionals;
  if (!setupPath || !sigPath || !tag) {
    throw new Error(
      "Usage: node scripts/make-latest-json.mjs <setup.exe> <setup.exe.sig> <tag> [notes] " +
        "[--notes-file <path>] [--config <tauri.conf.json>] [--out <latest.json>]",
    );
  }
  return { setupPath, sigPath, tag, notes, ...flags };
}

export function main(argv, { now = () => new Date(), log = console.log } = {}) {
  const args = parseArgs(argv);

  if (!existsSync(args.setupPath)) {
    throw new Error(`Setup installer not found: ${args.setupPath}`);
  }
  if (!existsSync(args.sigPath)) {
    throw new Error(
      `Signature file not found: ${args.sigPath}. ` +
        `\`tauri build\` only emits .sig files when bundle.createUpdaterArtifacts is ` +
        `enabled and TAURI_SIGNING_PRIVATE_KEY is set — check the signing secrets.`,
    );
  }

  const config = JSON.parse(readFileSync(args.config, "utf8"));
  const version = assertTagMatchesVersion(args.tag, config.version);

  const signature = readFileSync(args.sigPath, "utf8").trim();
  if (!signature) {
    throw new Error(`Signature file is empty: ${args.sigPath}`);
  }

  let notes = args.notes;
  if (args.notesFile) notes = readFileSync(args.notesFile, "utf8").trim();
  if (!notes) notes = `Lore Desktop ${args.tag}`;

  const manifest = buildLatestJson({
    version,
    notes,
    pubDate: now().toISOString(),
    signature,
    url: assetUrl(args.tag, basename(args.setupPath)),
  });

  writeFileSync(args.out, JSON.stringify(manifest, null, 2) + "\n", "utf8");
  log(`Wrote ${args.out} for ${version} -> ${manifest.platforms["windows-x86_64"].url}`);
  return manifest;
}

const isDirectRun =
  process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href;
if (isDirectRun) {
  try {
    main(process.argv.slice(2));
  } catch (err) {
    console.error(`make-latest-json: ${err.message}`);
    process.exit(1);
  }
}
