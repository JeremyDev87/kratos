import path from "node:path";

import { SOURCE_EXTENSIONS } from "./constants.js";
import { statOrNull, walkDirectory } from "./fs.js";

export async function collectSourceFiles(config) {
  const discovered = new Set();

  for (const root of config.roots) {
    const rootStat = await statOrNull(root);

    if (!rootStat?.isDirectory()) {
      continue;
    }

    const files = await walkDirectory(root, {
      ignoredDirectories: config.ignoredDirectories,
    });

    for (const file of files) {
      if (SOURCE_EXTENSIONS.includes(path.extname(file))) {
        discovered.add(path.resolve(file));
      }
    }
  }

  return [...discovered].sort();
}
