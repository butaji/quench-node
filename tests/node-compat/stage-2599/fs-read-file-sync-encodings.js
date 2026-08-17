"use strict";

const assert = require("assert");
const fs = require("fs");
const os = require("os");
const path = require("path");

const dir = fs.mkdtempSync(path.join(os.tmpdir(), "quench-read-enc-"));

// readFileSync returns a decoded string for every valid encoding and is
// case-insensitive for the encoding name.
const ascii = path.join(dir, "a.txt");
fs.writeFileSync(ascii, "abc");
assert.strictEqual(fs.readFileSync(ascii, "utf8"), "abc");
assert.strictEqual(fs.readFileSync(ascii, "UTF-8"), "abc");
assert.strictEqual(fs.readFileSync(ascii, { encoding: "ascii" }), "abc");
assert.strictEqual(fs.readFileSync(ascii, { encoding: "latin1" }), "abc");
assert.strictEqual(fs.readFileSync(ascii, { encoding: "base64" }), "YWJj");
assert.strictEqual(fs.readFileSync(ascii, { encoding: "hex" }), "616263");

// An empty append-mode read yields the empty string for each encoding, and a
// valid encoding (or none) still reads a non-empty file.
for (const enc of [
  "utf8",
  "ascii",
  "base64",
  "hex",
  "latin1",
  "uTf8",
  "utf16le"
]) {
  const empty = path.join(dir, `empty-${enc}.txt`);
  assert.strictEqual(
    fs.readFileSync(empty, { encoding: enc, flag: "a+" }),
    "",
    `encoding ${enc}`
  );
}

console.log("fs readFileSync encodings passed");
