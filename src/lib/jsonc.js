export function parseLooseJson(input) {
  const withoutComments = stripComments(input);
  const withoutTrailingCommas = stripTrailingCommas(withoutComments);
  return JSON.parse(withoutTrailingCommas);
}

function stripComments(input) {
  let output = "";
  let index = 0;
  let state = "code";

  while (index < input.length) {
    const current = input[index];
    const next = input[index + 1];

    if (state === "line-comment") {
      if (current === "\n") {
        state = "code";
        output += "\n";
      } else {
        output += " ";
      }

      index += 1;
      continue;
    }

    if (state === "block-comment") {
      if (current === "*" && next === "/") {
        output += "  ";
        index += 2;
        state = "code";
      } else {
        output += current === "\n" ? "\n" : " ";
        index += 1;
      }

      continue;
    }

    if (state === "string") {
      output += current;

      if (current === "\\" && next) {
        output += next;
        index += 2;
        continue;
      }

      if (current === "\"") {
        state = "code";
      }

      index += 1;
      continue;
    }

    if (current === "/" && next === "/") {
      output += "  ";
      index += 2;
      state = "line-comment";
      continue;
    }

    if (current === "/" && next === "*") {
      output += "  ";
      index += 2;
      state = "block-comment";
      continue;
    }

    if (current === "\"") {
      state = "string";
      output += current;
      index += 1;
      continue;
    }

    output += current;
    index += 1;
  }

  return output;
}

function stripTrailingCommas(input) {
  let output = "";
  let index = 0;
  let state = "code";

  while (index < input.length) {
    const current = input[index];

    if (state === "string") {
      output += current;

      if (current === "\\" && input[index + 1]) {
        output += input[index + 1];
        index += 2;
        continue;
      }

      if (current === "\"") {
        state = "code";
      }

      index += 1;
      continue;
    }

    if (current === "\"") {
      state = "string";
      output += current;
      index += 1;
      continue;
    }

    if (current === ",") {
      let lookahead = index + 1;

      while (lookahead < input.length && /\s/.test(input[lookahead])) {
        lookahead += 1;
      }

      if (input[lookahead] === "}" || input[lookahead] === "]") {
        index += 1;
        continue;
      }
    }

    output += current;
    index += 1;
  }

  return output;
}
