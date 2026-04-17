#!/usr/bin/env node

import { runClean } from "./commands/clean.js";
import { runReport } from "./commands/report.js";
import { runScan } from "./commands/scan.js";

const [, , command, ...args] = process.argv;

const HELP_TEXT = `Kratos
Destroy dead code ruthlessly.

Usage:
  kratos scan [root] [--output path] [--json]
  kratos report [report-path-or-root] [--format summary|json|md]
  kratos clean [report-path-or-root] [--apply]

Commands:
  scan    Analyze a codebase and save the latest report.
  report  Print a saved report in summary, json, or markdown form.
  clean   Show deletion candidates or delete them with --apply.
`;

async function main() {
  switch (command) {
    case undefined:
    case "--help":
    case "-h":
    case "help":
      console.log(HELP_TEXT);
      return;
    case "scan":
      await runScan(args);
      return;
    case "report":
      await runReport(args);
      return;
    case "clean":
      await runClean(args);
      return;
    default:
      console.error(`Unknown command: ${command}`);
      console.error("");
      console.error(HELP_TEXT);
      process.exitCode = 1;
  }
}

main().catch((error) => {
  const message = error instanceof Error ? error.message : String(error);
  console.error(`Kratos failed: ${message}`);
  process.exitCode = 1;
});
