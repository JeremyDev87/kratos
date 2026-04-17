#!/usr/bin/env node

import { cleanCommand } from "./commands/clean.js";
import { reportCommand } from "./commands/report.js";
import { scanCommand } from "./commands/scan.js";
import {
  formatCommandHelp,
  formatRootHelp,
  formatUnknownCommand,
} from "./lib/help.js";

const [, , command, ...args] = process.argv;
const COMMANDS = [scanCommand, reportCommand, cleanCommand];
const COMMANDS_BY_NAME = new Map(COMMANDS.map((entry) => [entry.name, entry]));

async function main() {
  switch (command) {
    case undefined:
    case "--help":
    case "-h":
    case "help":
      console.log(formatRootHelp(COMMANDS));
      return;
    default:
      break;
  }

  const commandEntry = COMMANDS_BY_NAME.get(command);

  if (!commandEntry) {
    console.error(formatUnknownCommand(command, COMMANDS));
    process.exitCode = 1;
    return;
  }

  if (shouldShowCommandHelp(args)) {
    console.log(formatCommandHelp(commandEntry));
    return;
  }

  await commandEntry.run(args);
}

main().catch((error) => {
  const message = error instanceof Error ? error.message : String(error);
  console.error(`Kratos failed: ${message}`);
  process.exitCode = 1;
});

function shouldShowCommandHelp(args) {
  return args.includes("--help") || (args.length === 1 && args[0] === "-h");
}
