"use strict";

const assert = require("assert");
const path = require("path");

// win32 splits on both separators and skips a leading drive root.
const winCases = [
  ["C:folder\\file.ext", ".ext"],
  ["C:file.ext", ".ext"],
  ["C:.file", ""],
  ["C:", ""],
  ["C:\\", ""],
  [".\\", ""],
  ["..\\", ""],
  ["file.ext\\", ".ext"],
  ["file.ext\\\\", ".ext"],
  ["file.\\", "."],
  ["file.", "."],
  [".file", ""],
  [".file.", "."],
  ["..file", ".file"],
  ["..file..", "."],
  ["...", "."],
  ["...ext", ".ext"]
];
for (const [input, want] of winCases) {
  assert.strictEqual(
    path.win32.extname(input),
    want,
    `win32 extname(${input})`
  );
}

// posix treats backslash as an ordinary character, not a separator.
const posCases = [
  ["..\\", ".\\"],
  ["file.ext\\", ".ext\\"],
  ["file.\\", ".\\"],
  [".\\", ""],
  ["file.ext", ".ext"],
  [".file", ""],
  [".file.ext", ".ext"],
  ["file.", "."],
  [".", ""],
  ["..", ""],
  ["file.ext/", ".ext"],
  ["/path/to/file", ""],
  ["/path/to/file.ext", ".ext"]
];
for (const [input, want] of posCases) {
  assert.strictEqual(
    path.posix.extname(input),
    want,
    `posix extname(${input})`
  );
}

console.log("path extname platforms passed");
