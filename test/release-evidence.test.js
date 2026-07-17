import test from "node:test";
import assert from "node:assert/strict";

import {
  assertPublishedDistTag,
  assertPublishedReleaseDistTags,
  releasePackageNames,
  resolveReleasePlan,
} from "../scripts/lib/release.mjs";

test("release evidence lists root and platform addon packages", () => {
  const names = releasePackageNames({
    name: "@jeremyfellaz/kratos",
    optionalDependencies: {
      "@jeremyfellaz/kratos-darwin-arm64": "0.3.7",
      "@jeremyfellaz/kratos-win32-x64-msvc": "0.3.7",
    },
  });

  assert.deepEqual(names, [
    "@jeremyfellaz/kratos",
    "@jeremyfellaz/kratos-darwin-arm64",
    "@jeremyfellaz/kratos-win32-x64-msvc",
  ]);
  assert.deepEqual(releasePackageNames({ name: "@jeremyfellaz/kratos" }), ["@jeremyfellaz/kratos"]);
  assert.throws(() => releasePackageNames({}), /must have a package name/);
});

test("published release audit validates every package for stable and prerelease tags", () => {
  const manifest = {
    name: "@jeremyfellaz/kratos",
    optionalDependencies: {
      "@jeremyfellaz/kratos-darwin-arm64": "0.3.7",
      "@jeremyfellaz/kratos-win32-x64-msvc": "0.3.7",
    },
  };
  const stableLookups = [];

  assert.deepEqual(
    assertPublishedReleaseDistTags(manifest, "v0.3.7", (name, version) => {
      stableLookups.push(`${name}@${version}`);
      return { latest: version };
    }),
    { tag: "v0.3.7", version: "0.3.7", npmDistTag: "latest" },
  );
  assert.equal(stableLookups.length, 3);
  assert.throws(
    () => assertPublishedReleaseDistTags(manifest, "v0.4.0-beta.1", () => ({ next: "0.4.0-beta.0" })),
    /expected 0\.4\.0-beta\.1, got 0\.4\.0-beta\.0/,
  );
});

test("published dist-tag must point at the expected release version", () => {
  const plan = resolveReleasePlan("v0.3.7");

  assert.doesNotThrow(() =>
    assertPublishedDistTag({ latest: "0.3.7", next: "0.4.0-beta.1" }, plan.npmDistTag, plan.version),
  );
  assert.throws(
    () => assertPublishedDistTag({ latest: "0.3.6" }, plan.npmDistTag, plan.version),
    /expected 0\.3\.7, got 0\.3\.6/,
  );
  assert.throws(
    () => assertPublishedDistTag({}, plan.npmDistTag, plan.version),
    /does not expose dist-tag latest/,
  );
  assert.throws(
    () => assertPublishedDistTag(null, plan.npmDistTag, plan.version),
    /must be a JSON object/,
  );
});
