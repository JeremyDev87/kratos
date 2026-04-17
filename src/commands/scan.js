import path from "node:path";

import { parseCliOptions } from "../lib/args.js";
import { analyzeProject } from "../lib/analyze.js";
import { DEFAULT_REPORT_RELATIVE_PATH } from "../lib/constants.js";
import { ensureDir, writeJsonFile } from "../lib/fs.js";
import { formatSummaryReport } from "../lib/report.js";

export async function runScan(argv) {
  const { positionals, flags } = parseCliOptions(argv);
  const root = path.resolve(positionals[0] ?? process.cwd());
  const outputPath = resolveOutputPath(root, flags.output);
  const report = await analyzeProject(root);

  await ensureDir(path.dirname(outputPath));
  await writeJsonFile(outputPath, report);

  if (flags.json) {
    console.log(JSON.stringify(report, null, 2));
    return;
  }

  console.log(formatSummaryReport(report, outputPath, "Kratos scan complete."));
}

function resolveOutputPath(root, outputFlag) {
  if (!outputFlag) {
    return path.join(root, DEFAULT_REPORT_RELATIVE_PATH);
  }

  return path.isAbsolute(outputFlag)
    ? outputFlag
    : path.resolve(root, outputFlag);
}
