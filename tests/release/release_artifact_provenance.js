const crypto = require("crypto");
const fs = require("fs");
const path = require("path");
const { execFileSync } = require("child_process");

function exactCommit(root, env) {
  let commit = env.SCENA_RELEASE_COMMIT || env.GITHUB_SHA;
  if (!commit) {
    try {
      commit = execFileSync("git", ["rev-parse", "HEAD"], {
        cwd: root,
        encoding: "utf8",
        stdio: ["ignore", "pipe", "ignore"],
      }).trim();
    } catch (_error) {
      throw new Error(
        "release artifacts require SCENA_RELEASE_COMMIT, GITHUB_SHA, or a Git checkout",
      );
    }
  }
  if (!/^[0-9a-fA-F]{40}$/.test(commit)) {
    throw new Error(
      `release artifact commit must be exactly 40 hexadecimal characters, got ${JSON.stringify(commit)}`,
    );
  }
  return commit.toLowerCase();
}

function sourceChecksum(root, relative) {
  if (!relative || path.isAbsolute(relative)) {
    throw new Error(`release provenance source must be a relative path: ${relative}`);
  }
  const canonicalRoot = path.resolve(root);
  const file = path.resolve(canonicalRoot, relative);
  if (!file.startsWith(`${canonicalRoot}${path.sep}`)) {
    throw new Error(`release provenance source escapes repository root: ${relative}`);
  }
  return {
    path: relative.split(path.sep).join("/"),
    sha256: crypto.createHash("sha256").update(fs.readFileSync(file)).digest("hex"),
  };
}

function attachReleaseArtifactProvenance(artifact, options) {
  if (!artifact || Array.isArray(artifact) || typeof artifact !== "object") {
    throw new Error("release artifact must be a JSON object");
  }
  const { root, schema, producer, sourcePaths, env = process.env } = options;
  if (typeof schema !== "string" || schema.trim() === "") {
    throw new Error("release artifact schema must be non-blank");
  }
  if (typeof producer !== "string" || producer.trim() === "") {
    throw new Error("release artifact producer must be non-blank");
  }
  if (!Array.isArray(sourcePaths) || sourcePaths.length === 0) {
    throw new Error("release artifact must bind at least one producer source");
  }
  return {
    ...artifact,
    schema,
    producer,
    commit_sha: exactCommit(root, env),
    timestamp_unix_seconds: Math.floor(Date.now() / 1000),
    source_checksums: sourcePaths.map((relative) => sourceChecksum(root, relative)),
  };
}

module.exports = { attachReleaseArtifactProvenance };
