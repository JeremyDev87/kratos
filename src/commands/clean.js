import path from "node:path";

import { parseCliOptions } from "../lib/args.js";
import {
  fileExists,
  isWithinDirectory,
  realpathOrNull,
  readJsonFile,
  removeEmptyDirectories,
  removeFile,
} from "../lib/fs.js";
import { resolveReportInput } from "../lib/report.js";

export const cleanCommand = {
  name: "clean",
  summary: "Show deletion candidates or delete them with --apply.",
  usage: ["kratos clean [report-path-or-root] [--apply]"],
  run: runClean,
};

export async function runClean(argv) {
  const { positionals, flags } = parseCliOptions(argv);
  const reportPath = resolveReportInput(positionals[0], process.cwd());
  const report = await readJsonFile(reportPath);
  const candidates = report.findings.deletionCandidates ?? [];
  const reportRootPath = path.resolve(report.root);
  const deletionRoot =
    (await realpathOrNull(report.root)) ?? path.resolve(report.root);

  if (!candidates.length) {
    console.log("Kratos clean found no deletion candidates.");
    return;
  }

  if (!flags.apply) {
    console.log("Kratos clean dry run.");
    console.log("");

    for (const candidate of candidates) {
      console.log(`- ${candidate.file} (${candidate.reason})`);
    }

    console.log("");
    console.log("Re-run with --apply to delete these files.");
    return;
  }

  let deleted = 0;

  for (const candidate of candidates) {
    const candidatePath = path.resolve(candidate.file);

    if (!isWithinDirectory(reportRootPath, candidatePath)) {
      continue;
    }

    if (!(await fileExists(candidatePath))) {
      continue;
    }

    const candidateParent =
      (await realpathOrNull(path.dirname(candidatePath))) ??
      path.resolve(path.dirname(candidatePath));

    if (!isWithinDirectory(deletionRoot, candidateParent)) {
      continue;
    }

    await removeFile(candidatePath);
    await removeEmptyDirectories(path.dirname(candidatePath), reportRootPath);
    deleted += 1;
  }

  console.log(`Kratos clean deleted ${deleted} file(s).`);
}
