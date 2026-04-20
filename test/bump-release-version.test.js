import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";

import {
  bumpPackageVersion,
  normalizeReleaseTag,
  updatePackageManifest,
} from "../scripts/bump-release-version.mjs";

test("normalizeReleaseTag accepts bare versions", () => {
  assert.equal(normalizeReleaseTag("1.2.3"), "v1.2.3");
  assert.equal(normalizeReleaseTag("v1.2.3-beta.1"), "v1.2.3-beta.1");
});

test("updatePackageManifest syncs root and optional dependency versions", () => {
  const pkg = {
    name: "kratos",
    version: "0.2.0-alpha.1",
    optionalDependencies: {
      "@kratos/darwin-arm64": "0.2.0-alpha.1",
      "@kratos/linux-x64-gnu": "0.2.0-alpha.1",
    },
  };

  const updated = updatePackageManifest(pkg, "0.2.0");

  assert.equal(updated.version, "0.2.0");
  assert.deepEqual(updated.optionalDependencies, {
    "@kratos/darwin-arm64": "0.2.0",
    "@kratos/linux-x64-gnu": "0.2.0",
  });
});

test("bumpPackageVersion rewrites package.json from tag input", async () => {
  const tempRoot = await fs.mkdtemp(path.join(os.tmpdir(), "kratos-bump-version-"));
  const manifestPath = path.join(tempRoot, "package.json");

  await fs.writeFile(
    manifestPath,
    `${JSON.stringify(
      {
        name: "kratos",
        version: "0.2.0-alpha.1",
        optionalDependencies: {
          "@kratos/darwin-arm64": "0.2.0-alpha.1",
          "@kratos/win32-x64-msvc": "0.2.0-alpha.1",
        },
      },
      null,
      2,
    )}\n`,
  );

  const result = await bumpPackageVersion("v0.2.0", manifestPath);
  const updated = JSON.parse(await fs.readFile(manifestPath, "utf8"));

  assert.equal(result.version, "0.2.0");
  assert.equal(result.tag, "v0.2.0");
  assert.equal(updated.version, "0.2.0");
  assert.deepEqual(updated.optionalDependencies, {
    "@kratos/darwin-arm64": "0.2.0",
    "@kratos/win32-x64-msvc": "0.2.0",
  });
});
