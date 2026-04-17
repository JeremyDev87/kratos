import path from "node:path";
import fs from "node:fs/promises";

export async function ensureDir(targetPath) {
  await fs.mkdir(targetPath, { recursive: true });
}

export async function fileExists(targetPath) {
  try {
    await fs.access(targetPath);
    return true;
  } catch {
    return false;
  }
}

export async function readTextFile(filePath) {
  return fs.readFile(filePath, "utf8");
}

export async function writeJsonFile(filePath, value) {
  await fs.writeFile(filePath, `${JSON.stringify(value, null, 2)}\n`, "utf8");
}

export async function readJsonFile(filePath) {
  const content = await readTextFile(filePath);
  return JSON.parse(content);
}

export async function statOrNull(targetPath) {
  try {
    return await fs.stat(targetPath);
  } catch {
    return null;
  }
}

export async function realpathOrNull(targetPath) {
  try {
    return await fs.realpath(targetPath);
  } catch {
    return null;
  }
}

export async function walkDirectory(root, options = {}) {
  const discovered = [];
  const ignoredDirectories = new Set(options.ignoredDirectories ?? []);

  async function visit(currentPath) {
    const entries = await fs.readdir(currentPath, { withFileTypes: true });

    for (const entry of entries) {
      const fullPath = `${currentPath}/${entry.name}`;

      if (entry.isDirectory()) {
        if (ignoredDirectories.has(entry.name)) {
          continue;
        }

        await visit(fullPath);
        continue;
      }

      if (entry.isFile()) {
        discovered.push(fullPath);
      }
    }
  }

  await visit(root);
  return discovered;
}

export async function removeFile(filePath) {
  await fs.unlink(filePath);
}

export async function removeEmptyDirectories(startDir, stopAt) {
  const boundary = path.resolve(stopAt);
  let current = path.resolve(startDir);

  while (isWithinDirectory(boundary, current) && current !== boundary) {
    const entries = await fs.readdir(current).catch(() => null);

    if (!entries || entries.length > 0) {
      return;
    }

    await fs.rmdir(current).catch(() => null);
    current = path.dirname(current);
  }
}

export function isWithinDirectory(root, candidate) {
  const normalizedRoot = path.resolve(root);
  const normalizedCandidate = path.resolve(candidate);
  const relative = path.relative(normalizedRoot, normalizedCandidate);

  return (
    relative === "" ||
    (!relative.startsWith("..") && !path.isAbsolute(relative))
  );
}
