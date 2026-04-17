import { parseCliOptions } from "../lib/args.js";
import { readJsonFile } from "../lib/fs.js";
import {
  formatMarkdownReport,
  formatSummaryReport,
  resolveReportInput,
} from "../lib/report.js";

export const reportCommand = {
  name: "report",
  summary: "Print a saved report in summary, json, or markdown form.",
  usage: ["kratos report [report-path-or-root] [--format summary|json|md]"],
  run: runReport,
};

export async function runReport(argv) {
  const { positionals, flags } = parseCliOptions(argv);
  const reportPath = resolveReportInput(positionals[0], process.cwd());
  const report = await readJsonFile(reportPath);
  const format = String(flags.format ?? "summary");

  if (format === "json") {
    console.log(JSON.stringify(report, null, 2));
    return;
  }

  if (format === "md") {
    console.log(formatMarkdownReport(report, reportPath));
    return;
  }

  console.log(formatSummaryReport(report, reportPath, "Kratos report."));
}
