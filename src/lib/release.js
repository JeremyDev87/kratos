const RELEASE_TAG_PATTERN = /^v[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?$/;

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
