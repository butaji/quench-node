"use strict";

const assert = require("assert");
const path = require("path");

// posix normalize: collapse separators and "." segments, resolve "..".
assert.strictEqual(
  path.posix.normalize("./fixtures///b/../b/c.js"),
  "fixtures/b/c.js"
);
assert.strictEqual(path.posix.normalize("/foo/../../../bar"), "/bar");
assert.strictEqual(path.posix.normalize("a//b//../b"), "a/b");
assert.strictEqual(path.posix.normalize("a//b//./c"), "a/b/c");
assert.strictEqual(path.posix.normalize("/a/b/c/../../../x/y/z"), "/x/y/z");
assert.strictEqual(path.posix.normalize(""), ".");
assert.strictEqual(path.posix.normalize("//server/x"), "/server/x");

// win32 normalize recognizes drive roots, UNC shares, and device roots.
assert.strictEqual(path.win32.normalize("a//b//../b"), "a\\b");
assert.strictEqual(
  path.win32.normalize("//server/share/dir/file.ext"),
  "\\\\server\\share\\dir\\file.ext"
);
assert.strictEqual(path.win32.normalize("/foo/../../../bar"), "\\bar");
assert.strictEqual(path.win32.normalize("C:"), "C:.");
assert.strictEqual(path.win32.normalize("C:\\.\\"), "C:\\");
assert.strictEqual(path.win32.normalize("C:..\\abc"), "C:..\\abc");
assert.strictEqual(
  path.win32.normalize("C:..\\..\\abc\\..\\def"),
  "C:..\\..\\def"
);
assert.strictEqual(path.win32.normalize("file:stream"), "file:stream");
assert.strictEqual(path.win32.normalize("bar\\foo..\\..\\"), "bar\\");
assert.strictEqual(path.win32.normalize("bar\\foo..\\..\\baz"), "bar\\baz");
assert.strictEqual(
  path.win32.normalize("\\\\unc\\share\\foo"),
  "\\\\unc\\share\\foo"
);
assert.strictEqual(path.win32.normalize("\\\\.\\foo"), "\\\\.\\foo");

// CVE-2024-36139: a relative path must not turn into a drive-absolute path.
assert.strictEqual(
  path.win32.normalize("test/../C:/Windows"),
  ".\\C:\\Windows"
);
assert.strictEqual(
  path.win32.normalize("./upload/../C:/Windows"),
  ".\\C:\\Windows"
);
assert.strictEqual(
  path.win32.normalize("test/../??/D:/Test"),
  ".\\??\\D:\\Test"
);
assert.strictEqual(path.win32.normalize("test/C:/../../F:\\"), ".\\F:\\");

console.log("path normalize platforms passed");
