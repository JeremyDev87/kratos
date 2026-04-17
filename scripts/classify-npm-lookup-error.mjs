#!/usr/bin/env node

import fs from "node:fs";

import { classifyNpmLookupError } from "../src/lib/release.js";

const errorPath = process.argv[2];

if (!errorPath) {
  console.error("Usage: node ./scripts/classify-npm-lookup-error.mjs <stderr-file>");
  process.exit(1);
}

const stderr = fs.readFileSync(errorPath, "utf8");
console.log(classifyNpmLookupError(stderr));
