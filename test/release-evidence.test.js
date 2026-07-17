import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

import {
  assertPublishedDistTag,
  assertPublishedReleaseDistTags,
  releasePackageNames,
  resolveReleasePlan,
} from "../scripts/lib/release.mjs";

const publishedFollowUpWorkflow = readFileSync(
  new URL("../.github/workflows/release-published-follow-up.yml", import.meta.url),
  "utf8",
);
const ciWorkflow = readFileSync(new URL("../.github/workflows/ci.yml", import.meta.url), "utf8");

test("published follow-up validates a tag with trusted code before target checkout", () => {
  const trustedCheckout = publishedFollowUpWorkflow.indexOf("- name: Checkout trusted release helper");
  const resolveTarget = publishedFollowUpWorkflow.indexOf("- name: Resolve target release");
  const validateTag = publishedFollowUpWorkflow.indexOf('node ./scripts/release-plan.mjs "$tag"');
  const targetCheckout = publishedFollowUpWorkflow.indexOf("- name: Checkout published release tag");

  assert.ok(trustedCheckout >= 0, "trusted helper checkout must exist");
  assert.ok(resolveTarget > trustedCheckout, "target resolution must follow trusted checkout");
  assert.ok(validateTag > resolveTarget, "canonical tag validation must run during target resolution");
  assert.ok(targetCheckout > validateTag, "target checkout must follow canonical tag validation");
  assert.match(
    publishedFollowUpWorkflow,
    /ref: refs\/tags\/\$\{\{ steps\.meta\.outputs\.tag \}\}/,
    "target checkout must select an explicit tag ref",
  );
});

test("PR CI runs the release evidence contract suite", () => {
  assert.match(
    ciWorkflow,
    /- name: Run release evidence contract tests\n\s+run: node --test test\/release-evidence\.test\.js/,
  );
});

test("published follow-up passes event metadata through environment variables", () => {
  const resolveTarget = publishedFollowUpWorkflow.indexOf("- name: Resolve target release");
  const targetCheckout = publishedFollowUpWorkflow.indexOf("- name: Checkout published release tag");
  const resolveStep = publishedFollowUpWorkflow.slice(resolveTarget, targetCheckout);
  const runScript = resolveStep.slice(resolveStep.indexOf("run: |"));

  assert.match(resolveStep, /env:\n\s+EVENT_NAME: \$\{\{ github\.event_name \}\}/);
  assert.match(resolveStep, /INPUT_TAG: \$\{\{ inputs\.tag \}\}/);
  assert.match(resolveStep, /EVENT_RELEASE_TAG: \$\{\{ github\.event\.release\.tag_name \}\}/);
  assert.doesNotMatch(
    runScript,
    /\$\{\{ (?:inputs\.tag|github\.event\.release\.(?:tag_name|html_url|published_at)) \}\}/,
  );
});

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
