import path from "node:path";

import { DEFAULT_REPORT_RELATIVE_PATH } from "./constants.js";

export function resolveReportInput(input, cwd) {
  if (!input) {
    return path.resolve(cwd, DEFAULT_REPORT_RELATIVE_PATH);
  }

  const absolute = path.resolve(cwd, input);

  if (absolute.endsWith(".json")) {
    return absolute;
  }

  return path.join(absolute, DEFAULT_REPORT_RELATIVE_PATH);
}

export function formatSummaryReport(report, reportPath, title = "Kratos report.") {
  const lines = [
    title,
    "",
    `Root: ${report.root}`,
    `Files scanned: ${report.summary.filesScanned}`,
    `Entrypoints: ${report.summary.entrypoints}`,
    `Broken imports: ${report.summary.brokenImports}`,
    `Orphan files: ${report.summary.orphanFiles}`,
    `Dead exports: ${report.summary.deadExports}`,
    `Unused imports: ${report.summary.unusedImports}`,
    `Deletion candidates: ${report.summary.deletionCandidates}`,
    "",
    `Saved report: ${reportPath}`,
  ];

  appendPreview(lines, "Broken imports", report.findings.brokenImports, (item) =>
    `${item.file} -> ${item.source}`,
  );
  appendPreview(lines, "Orphan files", report.findings.orphanFiles, (item) => item.file);
  appendPreview(lines, "Dead exports", report.findings.deadExports, (item) =>
    `${item.file}#${item.exportName}`,
  );

  return lines.join("\n");
}

export function formatMarkdownReport(report, reportPath) {
  const lines = [
    "# Kratos Report",
    "",
    `- Generated: ${report.generatedAt}`,
    `- Root: ${report.root}`,
    `- Report: ${reportPath}`,
    "",
    "## Summary",
    "",
    `- Files scanned: ${report.summary.filesScanned}`,
    `- Entrypoints: ${report.summary.entrypoints}`,
    `- Broken imports: ${report.summary.brokenImports}`,
    `- Orphan files: ${report.summary.orphanFiles}`,
    `- Dead exports: ${report.summary.deadExports}`,
    `- Unused imports: ${report.summary.unusedImports}`,
    `- Deletion candidates: ${report.summary.deletionCandidates}`,
    "",
  ];

  pushMarkdownSection(lines, "Broken imports", report.findings.brokenImports, (item) =>
    `${item.file} -> \`${item.source}\``,
  );
  pushMarkdownSection(lines, "Orphan files", report.findings.orphanFiles, (item) =>
    `${item.file} (${item.reason})`,
  );
  pushMarkdownSection(lines, "Dead exports", report.findings.deadExports, (item) =>
    `${item.file} -> \`${item.exportName}\``,
  );
  pushMarkdownSection(lines, "Unused imports", report.findings.unusedImports, (item) =>
    `${item.file} -> \`${item.local}\` from \`${item.source}\``,
  );
  pushMarkdownSection(lines, "Deletion candidates", report.findings.deletionCandidates, (item) =>
    `${item.file} (${item.reason}, confidence ${item.confidence})`,
  );

  return lines.join("\n");
}

function appendPreview(lines, label, items, render) {
  if (!items?.length) {
    return;
  }

  lines.push("");
  lines.push(`${label}:`);

  for (const item of items.slice(0, 5)) {
    lines.push(`- ${render(item)}`);
  }

  if (items.length > 5) {
    lines.push(`- ...and ${items.length - 5} more`);
  }
}

function pushMarkdownSection(lines, title, items, render) {
  lines.push(`## ${title}`);
  lines.push("");

  if (!items?.length) {
    lines.push("- None");
    lines.push("");
    return;
  }

  for (const item of items) {
    lines.push(`- ${render(item)}`);
  }

  lines.push("");
}
