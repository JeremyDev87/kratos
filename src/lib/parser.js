const IDENTIFIER = /[A-Za-z_$][A-Za-z0-9_$]*/;

export function parseModuleSource(source) {
  const imports = [];
  const exports = [];
  const sourceWithoutComments = maskComments(source);
  const maskedSource = maskCommentsAndStrings(source);
  const removalRanges = [];

  collectStaticImports(sourceWithoutComments, imports, removalRanges);
  collectSideEffectImports(sourceWithoutComments, imports, removalRanges);
  collectReExports(sourceWithoutComments, imports, exports, removalRanges);
  collectRequireCalls(sourceWithoutComments, imports, removalRanges);
  collectDynamicImports(sourceWithoutComments, imports, removalRanges);
  collectExports(sourceWithoutComments, exports);

  const bodyWithoutImports = blankRanges(maskedSource, removalRanges);
  const unusedImports = detectUnusedImports(imports, bodyWithoutImports);

  return { imports, exports, unusedImports };
}

function collectStaticImports(source, imports, removalRanges) {
  const pattern = /^\s*import\s+(?!['"])([\s\S]*?)\s+from\s+['"]([^'"]+)['"];?/gm;

  for (const match of source.matchAll(pattern)) {
    const clause = match[1].trim();
    imports.push({
      source: match[2],
      kind: "static",
      specifiers: parseImportClause(clause),
    });
    removalRanges.push([match.index, match.index + match[0].length]);
  }
}

function collectSideEffectImports(source, imports, removalRanges) {
  const pattern = /^\s*import\s+['"]([^'"]+)['"];?/gm;

  for (const match of source.matchAll(pattern)) {
    imports.push({
      source: match[1],
      kind: "side-effect",
      specifiers: [],
    });
    removalRanges.push([match.index, match.index + match[0].length]);
  }
}

function collectReExports(source, imports, exports, removalRanges) {
  const pattern = /^\s*export\s+([\s\S]*?)\s+from\s+['"]([^'"]+)['"];?/gm;

  for (const match of source.matchAll(pattern)) {
    const clause = match[1].trim();

    if (clause === "*") {
      imports.push({
        source: match[2],
        kind: "reexport-all",
        specifiers: [{ kind: "unknown", imported: "*", local: "*" }],
      });
      exports.push({ name: "*", kind: "reexport-all" });
    } else if (/^\*\s+as\s+/.test(clause)) {
      const namespaceMatch = clause.match(/^\*\s+as\s+([A-Za-z_$][A-Za-z0-9_$]*)$/);

      if (namespaceMatch) {
        imports.push({
          source: match[2],
          kind: "reexport-namespace",
          specifiers: [{ kind: "namespace", imported: "*", local: namespaceMatch[1] }],
        });
        exports.push({ name: namespaceMatch[1], kind: "reexport-namespace" });
      }
    } else {
      const specifiers = parseNamedListClause(clause);
      imports.push({
        source: match[2],
        kind: "reexport",
        specifiers: specifiers.map((specifier) => ({
          kind: specifier.imported === "default" ? "default" : "named",
          imported: specifier.imported,
          local: specifier.local,
        })),
      });

      for (const specifier of specifiers) {
        exports.push({ name: specifier.local, kind: "reexport" });
      }
    }

    removalRanges.push([match.index, match.index + match[0].length]);
  }
}

function collectRequireCalls(source, imports, removalRanges) {
  const pattern =
    /\b(?:const|let|var)\s+([A-Za-z_$][A-Za-z0-9_$]*)\s*=\s*require\(\s*['"]([^'"]+)['"]\s*\)/g;

  for (const match of source.matchAll(pattern)) {
    imports.push({
      source: match[2],
      kind: "require",
      specifiers: [{ kind: "default", imported: "default", local: match[1].trim() }],
    });
    removalRanges.push([match.index, match.index + match[0].length]);
  }

  collectDestructuredRequireCalls(source, imports, removalRanges);
}

function collectDynamicImports(source, imports, removalRanges) {
  const pattern = /\bimport\(\s*['"]([^'"]+)['"]\s*\)/g;

  for (const match of source.matchAll(pattern)) {
    imports.push({
      source: match[1],
      kind: "dynamic",
      specifiers: [],
    });
    removalRanges.push([match.index, match.index + match[0].length]);
  }
}

function collectExports(source, exports) {
  const defaultPattern = /^\s*export\s+default\b/gm;
  const functionPattern =
    /^\s*export\s+(?:async\s+)?function\s+([A-Za-z_$][A-Za-z0-9_$]*)/gm;
  const classPattern = /^\s*export\s+class\s+([A-Za-z_$][A-Za-z0-9_$]*)/gm;
  const variablePattern = /^\s*export\s+(?:const|let|var)\s+([^;\n]+)/gm;
  const namedPattern = /^\s*export\s*{([^}]+)}(?!\s*from)/gm;
  const commonJsPattern = /\bexports\.([A-Za-z_$][A-Za-z0-9_$]*)\s*=/g;
  const moduleExportsPattern = /\bmodule\.exports\s*=/g;

  for (const _match of source.matchAll(defaultPattern)) {
    exports.push({ name: "default", kind: "default" });
  }

  for (const match of source.matchAll(functionPattern)) {
    exports.push({ name: match[1], kind: "named" });
  }

  for (const match of source.matchAll(classPattern)) {
    exports.push({ name: match[1], kind: "named" });
  }

  for (const match of source.matchAll(variablePattern)) {
    for (const name of extractVariableNames(match[1])) {
      exports.push({ name, kind: "named" });
    }
  }

  for (const match of source.matchAll(namedPattern)) {
    for (const specifier of parseNamedListClause(match[1])) {
      exports.push({ name: specifier.local, kind: "named" });
    }
  }

  for (const match of source.matchAll(commonJsPattern)) {
    exports.push({ name: match[1], kind: "named" });
  }

  for (const _match of source.matchAll(moduleExportsPattern)) {
    exports.push({ name: "default", kind: "default" });
  }
}

function parseImportClause(clause) {
  if (!clause) {
    return [];
  }

  const normalizedClause = clause.replace(/^type\s+/, "").trim();

  if (!normalizedClause) {
    return [];
  }

  if (normalizedClause.startsWith("{")) {
    return parseNamedListClause(normalizedClause).map((specifier) => ({
      kind: "named",
      imported: specifier.imported,
      local: specifier.local,
    }));
  }

  if (normalizedClause.startsWith("*")) {
    const namespaceMatch = normalizedClause.match(
      /^\*\s+as\s+([A-Za-z_$][A-Za-z0-9_$]*)$/,
    );
    return namespaceMatch
      ? [{ kind: "namespace", imported: "*", local: namespaceMatch[1] }]
      : [];
  }

  if (normalizedClause.includes(",")) {
    const [first, rest] = splitOnce(normalizedClause, ",");
    return [
      { kind: "default", imported: "default", local: first.trim() },
      ...parseImportClause(rest.trim()),
    ].filter((specifier) => specifier.local);
  }

  return [{ kind: "default", imported: "default", local: normalizedClause.trim() }];
}

function parseNamedListClause(clause) {
  const inner = clause.trim().replace(/^\{/, "").replace(/}$/, "").trim();

  if (!inner) {
    return [];
  }

  return inner
    .split(",")
    .map((part) => part.trim())
    .filter(Boolean)
    .map((part) => {
      const [imported, local] = splitSpecifier(part.replace(/^type\s+/, ""));
      return { imported, local };
    });
}

function splitSpecifier(part) {
  if (part.includes(" as ")) {
    const [left, right] = part.split(/\s+as\s+/);
    return [left.trim(), right.trim()];
  }

  return [part, part];
}

function splitOnce(value, token) {
  const index = value.indexOf(token);
  return [value.slice(0, index), value.slice(index + token.length)];
}

function extractVariableNames(segment) {
  const names = [];
  const pattern = /([A-Za-z_$][A-Za-z0-9_$]*)\s*(?::[^=,]+)?=/g;

  for (const match of segment.matchAll(pattern)) {
    names.push(match[1]);
  }

  return names;
}

function detectUnusedImports(imports, body) {
  const unusedImports = [];

  for (const entry of imports) {
    for (const specifier of entry.specifiers) {
      if (!specifier.local || specifier.kind === "unknown") {
        continue;
      }

      if (!IDENTIFIER.test(specifier.local)) {
        continue;
      }

      if (!hasIdentifierReference(body, specifier.local)) {
        unusedImports.push({
          source: entry.source,
          local: specifier.local,
          imported: specifier.imported,
        });
      }
    }
  }

  return unusedImports;
}

function hasIdentifierReference(body, identifier) {
  const escaped = escapeRegExp(identifier);
  const pattern = new RegExp(`(^|[^A-Za-z0-9_$])${escaped}([^A-Za-z0-9_$]|$)`, "m");
  return pattern.test(body);
}

function blankRanges(source, ranges) {
  if (!ranges.length) {
    return source;
  }

  const characters = source.split("");

  for (const [start, end] of ranges) {
    for (let index = start; index < end; index += 1) {
      if (characters[index] !== "\n") {
        characters[index] = " ";
      }
    }
  }

  return characters.join("");
}

function maskCommentsAndStrings(source) {
  let result = "";
  let index = 0;
  const stateStack = [{ type: "code" }];

  function currentState() {
    return stateStack[stateStack.length - 1];
  }

  while (index < source.length) {
    const current = source[index];
    const next = source[index + 1];
    const state = currentState();

    if (state.type === "line-comment") {
      if (current === "\n") {
        stateStack.pop();
        result += "\n";
      } else {
        result += " ";
      }

      index += 1;
      continue;
    }

    if (state.type === "block-comment") {
      if (current === "*" && next === "/") {
        result += "  ";
        index += 2;
        stateStack.pop();
      } else {
        result += current === "\n" ? "\n" : " ";
        index += 1;
      }

      continue;
    }

    if (state.type === "single-quote" || state.type === "double-quote") {
      if (current === "\\" && next) {
        result += "  ";
        index += 2;
        continue;
      }

      if (
        (state.type === "single-quote" && current === "'") ||
        (state.type === "double-quote" && current === "\"")
      ) {
        result += " ";
        index += 1;
        stateStack.pop();
        continue;
      }

      result += current === "\n" ? "\n" : " ";
      index += 1;
      continue;
    }

    if (state.type === "template") {
      if (current === "\\" && next) {
        result += "  ";
        index += 2;
        continue;
      }

      if (current === "`") {
        result += " ";
        index += 1;
        stateStack.pop();
        continue;
      }

      if (current === "$" && next === "{") {
        result += "  ";
        index += 2;
        stateStack.push({ type: "template-expression", braceDepth: 1 });
        continue;
      }

      result += current === "\n" ? "\n" : " ";
      index += 1;
      continue;
    }

    if (state.type === "template-expression") {
      if (current === "}") {
        result += current;
        state.braceDepth -= 1;
        index += 1;

        if (state.braceDepth === 0) {
          stateStack.pop();
        }

        continue;
      }

      if (current === "{") {
        result += current;
        state.braceDepth += 1;
        index += 1;
        continue;
      }
    }

    if (current === "/" && next === "/") {
      result += "  ";
      index += 2;
      stateStack.push({ type: "line-comment" });
      continue;
    }

    if (current === "/" && next === "*") {
      result += "  ";
      index += 2;
      stateStack.push({ type: "block-comment" });
      continue;
    }

    if (current === "'") {
      result += " ";
      index += 1;
      stateStack.push({ type: "single-quote" });
      continue;
    }

    if (current === "\"") {
      result += " ";
      index += 1;
      stateStack.push({ type: "double-quote" });
      continue;
    }

    if (current === "`") {
      result += " ";
      index += 1;
      stateStack.push({ type: "template" });
      continue;
    }

    result += current;
    index += 1;
  }

  return result;
}

function parseRequireBinding(binding) {
  if (!binding.startsWith("{")) {
    return [{ kind: "default", imported: "default", local: binding }];
  }

  const inner = binding.replace(/^\{/, "").replace(/}$/, "").trim();

  if (!inner) {
    return [];
  }

  if (/[{\[]/.test(inner) || inner.includes("...")) {
    return [{ kind: "unknown", imported: "*", local: "*" }];
  }

  return inner
    .split(",")
    .map((part) => part.trim())
    .filter(Boolean)
    .map(parseRequireBindingPart)
    .filter((specifier) => specifier.local);
}

function collectDestructuredRequireCalls(source, imports, removalRanges) {
  const declarationPattern = /\b(?:const|let|var)\s+\{/g;

  for (const match of source.matchAll(declarationPattern)) {
    const bindingStart = match.index + match[0].lastIndexOf("{");
    const bindingEnd = findMatchingBrace(source, bindingStart);

    if (bindingEnd === -1) {
      continue;
    }

    const binding = source.slice(bindingStart, bindingEnd + 1);
    const remainder = source.slice(bindingEnd + 1);
    const requireMatch = remainder.match(
      /^\s*=\s*require\(\s*['"]([^'"]+)['"]\s*\)\s*;?/,
    );

    if (!requireMatch) {
      continue;
    }

    imports.push({
      source: requireMatch[1],
      kind: "require",
      specifiers: parseRequireBinding(binding),
    });
    removalRanges.push([
      match.index,
      bindingEnd + 1 + requireMatch[0].length,
    ]);
  }
}

function findMatchingBrace(source, startIndex) {
  let depth = 0;
  let index = startIndex;
  let stringState = null;

  while (index < source.length) {
    const current = source[index];
    const next = source[index + 1];

    if (stringState) {
      if (current === "\\" && next) {
        index += 2;
        continue;
      }

      if (current === stringState) {
        stringState = null;
      }

      index += 1;
      continue;
    }

    if (current === "'" || current === "\"" || current === "`") {
      stringState = current;
      index += 1;
      continue;
    }

    if (current === "{") {
      depth += 1;
    } else if (current === "}") {
      depth -= 1;

      if (depth === 0) {
        return index;
      }
    }

    index += 1;
  }

  return -1;
}

function parseRequireBindingPart(part) {
  const cleaned = part.replace(/^\.{3}/, "").trim();
  const aliasIndex = findTopLevelToken(cleaned, ":");
  const defaultIndex = findTopLevelToken(cleaned, "=");

  if (aliasIndex !== -1 && (defaultIndex === -1 || aliasIndex < defaultIndex)) {
    const imported = cleaned.slice(0, aliasIndex).trim();
    const local = cleaned
      .slice(aliasIndex + 1)
      .trim()
      .split("=")[0]
      .trim();

    return {
      kind: "named",
      imported,
      local,
    };
  }

  const local = defaultIndex === -1
    ? cleaned
    : cleaned.slice(0, defaultIndex).trim();

  return {
    kind: "named",
    imported: local,
    local,
  };
}

function findTopLevelToken(source, token) {
  let depth = 0;
  let stringState = null;

  for (let index = 0; index < source.length; index += 1) {
    const current = source[index];
    const next = source[index + 1];

    if (stringState) {
      if (current === "\\" && next) {
        index += 1;
        continue;
      }

      if (current === stringState) {
        stringState = null;
      }

      continue;
    }

    if (current === "'" || current === "\"" || current === "`") {
      stringState = current;
      continue;
    }

    if (current === "(") {
      depth += 1;
      continue;
    }

    if (current === ")") {
      depth = Math.max(0, depth - 1);
      continue;
    }

    if (depth === 0 && current === token) {
      return index;
    }
  }

  return -1;
}

function maskComments(source) {
  let result = "";
  let index = 0;
  let state = "code";

  while (index < source.length) {
    const current = source[index];
    const next = source[index + 1];

    if (state === "line-comment") {
      if (current === "\n") {
        state = "code";
        result += "\n";
      } else {
        result += " ";
      }

      index += 1;
      continue;
    }

    if (state === "block-comment") {
      if (current === "*" && next === "/") {
        result += "  ";
        index += 2;
        state = "code";
      } else {
        result += current === "\n" ? "\n" : " ";
        index += 1;
      }

      continue;
    }

    if (state === "single-quote" || state === "double-quote" || state === "template") {
      const quote =
        state === "single-quote" ? "'" : state === "double-quote" ? "\"" : "`";

      result += current;

      if (current === "\\" && next) {
        result += next;
        index += 2;
        continue;
      }

      if (current === quote) {
        state = "code";
      }

      index += 1;
      continue;
    }

    if (current === "/" && next === "/") {
      result += "  ";
      index += 2;
      state = "line-comment";
      continue;
    }

    if (current === "/" && next === "*") {
      result += "  ";
      index += 2;
      state = "block-comment";
      continue;
    }

    if (current === "'") {
      result += current;
      index += 1;
      state = "single-quote";
      continue;
    }

    if (current === "\"") {
      result += current;
      index += 1;
      state = "double-quote";
      continue;
    }

    if (current === "`") {
      result += current;
      index += 1;
      state = "template";
      continue;
    }

    result += current;
    index += 1;
  }

  return result;
}

function escapeRegExp(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}
