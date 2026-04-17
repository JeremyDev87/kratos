export function formatRootHelp(commands) {
  const lines = [
    "Kratos",
    "Destroy dead code ruthlessly.",
    "",
    "Usage:",
    ...commands.flatMap((command) => command.usage.map((usageLine) => `  ${usageLine}`)),
    "",
    "Commands:",
    ...formatCommandSummaries(commands),
  ];

  return lines.join("\n");
}

export function formatCommandHelp(command) {
  return [
    "Kratos",
    "Destroy dead code ruthlessly.",
    "",
    `${command.name} command`,
    command.summary,
    "",
    "Usage:",
    ...command.usage.map((usageLine) => `  ${usageLine}`),
    "",
    "Run `kratos --help` to see every command.",
  ].join("\n");
}

export function formatUnknownCommand(commandName, commands) {
  return [`Unknown command: ${commandName}`, "", formatRootHelp(commands)].join("\n");
}

function formatCommandSummaries(commands) {
  const maxNameLength = commands.reduce(
    (longest, command) => Math.max(longest, command.name.length),
    0,
  );

  return commands.map(
    (command) => `  ${command.name.padEnd(maxNameLength)}  ${command.summary}`,
  );
}
