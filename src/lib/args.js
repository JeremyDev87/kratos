export function parseCliOptions(argv) {
  const positionals = [];
  const flags = {};

  for (let index = 0; index < argv.length; index += 1) {
    const token = argv[index];

    if (!token.startsWith("--")) {
      positionals.push(token);
      continue;
    }

    const withoutPrefix = token.slice(2);
    const [rawName, inlineValue] = withoutPrefix.split("=", 2);

    if (inlineValue !== undefined) {
      flags[rawName] = inlineValue;
      continue;
    }

    const nextToken = argv[index + 1];

    if (nextToken && !nextToken.startsWith("--")) {
      flags[rawName] = nextToken;
      index += 1;
      continue;
    }

    flags[rawName] = true;
  }

  return { positionals, flags };
}
