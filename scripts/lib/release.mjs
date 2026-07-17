const RELEASE_TAG_PATTERN = /^v[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?$/;
const RELEASE_VERSION_PATTERN = /^(?<major>[0-9]+)\.(?<minor>[0-9]+)\.(?<patch>[0-9]+)(?:-(?<prerelease>[0-9A-Za-z.-]+))?$/;

export function resolveReleasePlan(tag) {
  if (!RELEASE_TAG_PATTERN.test(tag)) {
    throw new Error("Tag must look like v1.2.3 or v1.2.3-beta.1");
  }

  const version = tag.slice(1);
  const isPrerelease = version.includes("-");

  return {
    tag,
    version,
    isPrerelease,
    npmDistTag: isPrerelease ? "next" : "latest",
    githubReleaseType: isPrerelease ? "prerelease" : "release",
  };
}

export function releasePackageNames(packageManifest) {
  if (!packageManifest || typeof packageManifest.name !== "string" || packageManifest.name.length === 0) {
    throw new Error("Release package manifest must have a package name");
  }

  return [packageManifest.name, ...Object.keys(packageManifest.optionalDependencies ?? {}).sort()];
}

export function assertPublishedDistTag(distTags, distTag, expectedVersion) {
  if (!distTags || Array.isArray(distTags) || typeof distTags !== "object") {
    throw new Error("npm dist-tags response must be a JSON object");
  }

  const actualVersion = distTags[distTag];
  if (typeof actualVersion !== "string") {
    throw new Error(`npm package does not expose dist-tag ${distTag}`);
  }
  if (actualVersion !== expectedVersion) {
    throw new Error(`npm dist-tag ${distTag} expected ${expectedVersion}, got ${actualVersion}`);
  }
}

export function assertPublishedReleaseDistTags(packageManifest, tag, readDistTags) {
  if (typeof readDistTags !== "function") {
    throw new Error("Release dist-tag audit needs a registry lookup function");
  }

  const plan = resolveReleasePlan(tag);
  for (const packageName of releasePackageNames(packageManifest)) {
    assertPublishedDistTag(readDistTags(packageName, plan.version), plan.npmDistTag, plan.version);
  }

  return { tag: plan.tag, version: plan.version, npmDistTag: plan.npmDistTag };
}

function parseReleaseVersion(version) {
  const match = RELEASE_VERSION_PATTERN.exec(version);

  if (!match || !match.groups) {
    throw new Error(`Version must look like 1.2.3 or 1.2.3-beta.1: ${version}`);
  }

  return {
    major: Number(match.groups.major),
    minor: Number(match.groups.minor),
    patch: Number(match.groups.patch),
    prerelease: match.groups.prerelease ? match.groups.prerelease.split(".") : [],
  };
}

function comparePrereleaseIdentifiers(left, right) {
  const leftIsNumeric = /^[0-9]+$/.test(left);
  const rightIsNumeric = /^[0-9]+$/.test(right);

  if (leftIsNumeric && rightIsNumeric) {
    return Number(left) === Number(right) ? 0 : Number(left) < Number(right) ? -1 : 1;
  }

  if (leftIsNumeric !== rightIsNumeric) {
    return leftIsNumeric ? -1 : 1;
  }

  if (left === right) {
    return 0;
  }

  return left < right ? -1 : 1;
}

export function compareReleaseVersions(leftVersion, rightVersion) {
  const left = parseReleaseVersion(leftVersion);
  const right = parseReleaseVersion(rightVersion);

  for (const key of ["major", "minor", "patch"]) {
    if (left[key] !== right[key]) {
      return left[key] < right[key] ? -1 : 1;
    }
  }

  if (left.prerelease.length === 0 && right.prerelease.length === 0) {
    return 0;
  }

  if (left.prerelease.length === 0) {
    return 1;
  }

  if (right.prerelease.length === 0) {
    return -1;
  }

  const length = Math.max(left.prerelease.length, right.prerelease.length);

  for (let index = 0; index < length; index += 1) {
    const leftIdentifier = left.prerelease[index];
    const rightIdentifier = right.prerelease[index];

    if (leftIdentifier === undefined) {
      return -1;
    }

    if (rightIdentifier === undefined) {
      return 1;
    }

    const comparison = comparePrereleaseIdentifiers(leftIdentifier, rightIdentifier);

    if (comparison !== 0) {
      return comparison;
    }
  }

  return 0;
}

export function assertReleaseUpgrade(currentVersion, nextVersion) {
  if (compareReleaseVersions(currentVersion, nextVersion) >= 0) {
    throw new Error(
      `Target version ${nextVersion} must be greater than current version ${currentVersion}`,
    );
  }
}

export function classifyNpmLookupError(stderr) {
  const normalized = stderr.toLowerCase();

  if (
    normalized.includes("e404") ||
    normalized.includes("404 not found") ||
    normalized.includes("is not in this registry")
  ) {
    return "not-found";
  }

  return "unknown";
}

export function classifyNpmPublishError(stderr) {
  const normalized = stderr.toLowerCase();

  if (
    normalized.includes("cannot publish over the previously published versions") ||
    normalized.includes("cannot publish over existing version") ||
    normalized.includes("previously published versions")
  ) {
    return "already-published";
  }

  return "unknown";
}
