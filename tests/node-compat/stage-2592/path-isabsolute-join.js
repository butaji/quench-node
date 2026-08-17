"use strict";

const assert = require("assert");
const path = require("path");

// win32 isAbsolute: leading separator or a drive root followed by a separator.
assert.strictEqual(path.win32.isAbsolute("/"), true);
assert.strictEqual(path.win32.isAbsolute("//server/file"), true);
assert.strictEqual(path.win32.isAbsolute("\\\\server"), true);
assert.strictEqual(path.win32.isAbsolute("\\\\"), true);
assert.strictEqual(path.win32.isAbsolute("c"), false);
assert.strictEqual(path.win32.isAbsolute("c:"), false);
assert.strictEqual(path.win32.isAbsolute("c:\\"), true);
assert.strictEqual(path.win32.isAbsolute("C:/Users/"), true);
assert.strictEqual(path.win32.isAbsolute("C:cwd\\another"), false);
assert.strictEqual(path.win32.isAbsolute("directory/directory"), false);

// posix isAbsolute: leading "/" only.
assert.strictEqual(path.posix.isAbsolute("/home/foo"), true);
assert.strictEqual(path.posix.isAbsolute("bar/"), false);
assert.strictEqual(path.posix.isAbsolute("./baz"), false);

// join: drop empty parts, join with the platform separator, then normalize.
assert.strictEqual(path.join(), ".");
assert.strictEqual(path.join(".", "x/b", "..", "/b/c.js"), "x/b/c.js");
assert.strictEqual(path.join("/foo", "../../../bar"), "/bar");
assert.strictEqual(path.join("foo", "../../../bar"), "../../bar");
assert.strictEqual(path.join("foo/x", "./bar"), "foo/x/bar");
assert.strictEqual(path.join("", "foo"), "foo");
assert.strictEqual(path.join("foo", "", "bar"), "foo/bar");
assert.strictEqual(path.join("./", "..", "/foo"), "../foo");

// win32 join: backslash separator, UNC slash collapse, drive roots.
assert.strictEqual(path.win32.join(".", "x/b", "..", "/b/c.js"), "x\\b\\c.js");
assert.strictEqual(path.win32.join("/foo", "../../../bar"), "\\bar");
assert.strictEqual(path.win32.join("foo", "../../../bar"), "..\\..\\bar");
assert.strictEqual(path.win32.join("foo/x/", ".", "bar"), "foo\\x\\bar");
assert.strictEqual(path.win32.join(""), ".");
assert.strictEqual(path.win32.join("foo", ""), "foo");
assert.strictEqual(path.win32.join("//server", "share"), "\\\\server\\share\\");

console.log("path isAbsolute/join passed");
