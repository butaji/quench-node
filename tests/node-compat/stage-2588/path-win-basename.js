"use strict";

const assert = require("assert");
const path = require("path");

// win32 drive-root handling: the "C:" prefix is not part of the basename and
// a separator directly after the root is not treated as a trailing separator.
assert.strictEqual(path.win32.basename("C:"), "");
assert.strictEqual(path.win32.basename("C:."), ".");
assert.strictEqual(path.win32.basename("C:\\"), "");
assert.strictEqual(path.win32.basename("C:/"), "");
assert.strictEqual(path.win32.basename("C:\\dir\\base.ext"), "base.ext");
assert.strictEqual(path.win32.basename("C:basename.ext"), "basename.ext");
assert.strictEqual(path.win32.basename("C:basename.ext\\"), "basename.ext");
assert.strictEqual(path.win32.basename("C:foo"), "foo");
assert.strictEqual(path.win32.basename("file:stream"), "file:stream");

// win32 treats both separators as equivalents.
assert.strictEqual(path.win32.basename("\\dir\\basename.ext"), "basename.ext");
assert.strictEqual(path.win32.basename("aaa/bbb/ccc"), "ccc");

// A suffix that equals the entire component is not stripped, matching Node.
assert.strictEqual(path.win32.basename("aaa\\bbb", "bbb"), "bbb");
assert.strictEqual(path.win32.basename("aaa\\bbb", "\\bbb"), "bbb");
assert.strictEqual(path.win32.basename("aaa\\bbb", "a\\bbb"), "bbb");

// A proper suffix is stripped.
assert.strictEqual(path.win32.basename("aaa\\bbb", "bb"), "b");
assert.strictEqual(path.win32.basename("aaa\\bbb", "b"), "bb");
assert.strictEqual(path.win32.basename("a", "a"), "");
assert.strictEqual(path.win32.basename("file.js", ".js"), "file");

// The same component/suffix semantics hold for posix.
assert.strictEqual(path.basename("aaa/bbb", "bbb"), "bbb");
assert.strictEqual(path.basename("a", "a"), "");
assert.strictEqual(path.basename(".js", ".js"), "");
assert.strictEqual(path.basename("js", ".js"), "js");
assert.strictEqual(path.basename("file.js", ".js"), "file");
assert.strictEqual(path.basename("file.js.old", ".js.old"), "file");
assert.strictEqual(path.basename("aaa/bbb/ccc"), "ccc");
assert.strictEqual(
  path.posix.basename("\\dir\\basename.ext"),
  "\\dir\\basename.ext"
);

console.log("path win32/posix basename passed");
