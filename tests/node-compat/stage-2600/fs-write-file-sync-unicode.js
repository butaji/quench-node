"use strict";

const assert = require("assert");
const fs = require("fs");
const os = require("os");
const path = require("path");

const dir = fs.mkdtempSync(path.join(os.tmpdir(), "quench-write-sd-"));

// writeFileSync accepts string data and UTF-8 encodes it, including
// multi-byte characters that the engine keeps as StringUnits.
const samples = [
  ["ascii", "abc"],
  ["ellipsis", "abc\u2026"],
  ["euro", "\u20ac"],
  ["cjk", "\u4f60\u597d"]
];
for (const [name, content] of samples) {
  const file = path.join(dir, `${name}.txt`);
  fs.writeFileSync(file, content);
  assert.strictEqual(fs.readFileSync(file, "utf8"), content, name);
}

console.log("fs writeFileSync unicode passed");
