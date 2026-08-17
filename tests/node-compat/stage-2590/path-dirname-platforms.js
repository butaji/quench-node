"use strict";

const assert = require("assert");
const path = require("path");

// posix dirname splits on "/" only.
const pos = [
  ["/a/b/", "/a"],
  ["/a/b", "/a"],
  ["/a", "/"],
  ["", "."],
  ["/", "/"],
  ["////", "/"],
  ["//a", "//"],
  ["foo", "."]
];
for (const [input, want] of pos) {
  assert.strictEqual(
    path.posix.dirname(input),
    want,
    `posix dirname(${input})`
  );
}

// win32 drives, relative drives, and backslash roots.
const win = [
  ["c:\\", "c:\\"],
  ["c:\\foo", "c:\\"],
  ["c:\\foo\\bar", "c:\\foo"],
  ["c:\\foo\\bar\\baz", "c:\\foo\\bar"],
  ["\\", "\\"],
  ["\\foo", "\\"],
  ["\\foo\\bar", "\\foo"],
  ["\\foo\\bar\\baz", "\\foo\\bar"],
  ["c:", "c:"],
  ["c:foo", "c:"],
  ["c:foo\\bar", "c:foo"],
  ["c:foo\\bar\\baz", "c:foo\\bar"],
  ["file:stream", "."],
  ["dir\\file:stream", "dir"],
  ["\\\\unc\\share", "\\\\unc\\share"],
  ["\\\\unc\\share\\foo", "\\\\unc\\share\\"],
  ["\\\\unc\\share\\foo\\bar", "\\\\unc\\share\\foo"],
  ["/a/b/", "/a"],
  ["foo", "."]
];
for (const [input, want] of win) {
  assert.strictEqual(
    path.win32.dirname(input),
    want,
    `win32 dirname(${input})`
  );
}

console.log("path dirname platforms passed");
