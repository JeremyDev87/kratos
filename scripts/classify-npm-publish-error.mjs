#!/usr/bin/env node

import fs from "node:fs";

import { classifyNpmPublishError } from "./lib/release.mjs";

const errorPath = process.argv[2];

if (!errorPath) {
  console.error("Usage: node ./scripts/classify-npm-publish-error.mjs <stderr-file>");
  process.exit(1);
}

const stderr = fs.readFileSync(errorPath, "utf8");
console.log(classifyNpmPublishError(stderr));
