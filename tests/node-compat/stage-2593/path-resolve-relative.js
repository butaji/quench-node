"use strict";

const assert = require("assert");
const path = require("path");

// posix resolve: joins against cwd (absolute) and collapses.
assert.strictEqual(path.posix.resolve("/var/lib", "../", "file/"), "/var/file");
assert.strictEqual(path.posix.resolve("/var/lib", "/../", "file/"), "/file");
assert.strictEqual(
  path.posix.resolve("/some/dir", ".", "/absolute/"),
  "/absolute"
);
assert.strictEqual(
  path.posix.resolve("/foo/tmp.3/", "../tmp.3/cycles/root.js"),
  "/foo/tmp.3/cycles/root.js"
);

// win32 resolve: device roots, drive-relative paths, UNC shares, device paths.
assert.strictEqual(
  path.win32.resolve("c:/blah\\blah", "d:/games", "c:../a"),
  "c:\\blah\\a"
);
assert.strictEqual(
  path.win32.resolve("c:/ignore", "d:\\a/b\\c/d", "\\e.exe"),
  "d:\\e.exe"
);
assert.strictEqual(
  path.win32.resolve("d:/ignore", "d:some/dir//"),
  "d:\\ignore\\some\\dir"
);
assert.strictEqual(
  path.win32.resolve("//server/share", "..", "relative\\"),
  "\\\\server\\share\\relative"
);
assert.strictEqual(path.win32.resolve("c:/", "//"), "c:\\");
assert.strictEqual(path.win32.resolve("c:/", "//dir"), "c:\\dir");
assert.strictEqual(
  path.win32.resolve("c:/", "//server/share"),
  "\\\\server\\share\\"
);
assert.strictEqual(
  path.win32.resolve("\\\\.\\PHYSICALDRIVE0"),
  "\\\\.\\PHYSICALDRIVE0"
);

// posix relative.
assert.strictEqual(path.posix.relative("/var/lib", "/var"), "..");
assert.strictEqual(path.posix.relative("/var/lib", "/bin"), "../../bin");
assert.strictEqual(path.posix.relative("/var/", "/var/lib"), "lib");
assert.strictEqual(path.posix.relative("/", "/var/lib"), "var/lib");
assert.strictEqual(
  path.posix.relative("/foo/bar/baz-quux", "/foo/bar/baz"),
  "../baz"
);
assert.strictEqual(path.posix.relative("/page1/page2/foo", "/"), "../../..");

// win32 relative: case-insensitive drives, UNC roots, device links.
assert.strictEqual(path.win32.relative("c:/aaaa/bbbb", "c:/aaaa"), "..");
assert.strictEqual(
  path.win32.relative("c:/aaaa/bbbb", "c:/cccc"),
  "..\\..\\cccc"
);
assert.strictEqual(path.win32.relative("c:/", "c:\\aaaa\\bbbb"), "aaaa\\bbbb");
assert.strictEqual(path.win32.relative("c:/AaAa/bbbb", "c:/aaaa/bbbb"), "");
assert.strictEqual(
  path.win32.relative("C:\\foo\\bar\\baz\\quux", "C:\\"),
  "..\\..\\..\\.."
);
assert.strictEqual(
  path.win32.relative("\\\\foo\\bar", "\\\\foo\\bar\\baz"),
  "baz"
);
assert.strictEqual(
  path.win32.relative("\\\\foo\\bar\\baz", "\\\\foo\\bar"),
  ".."
);
assert.strictEqual(
  path.win32.relative("C:\\baz", "\\\\foo\\bar\\baz"),
  "\\\\foo\\bar\\baz"
);

console.log("path resolve/relative passed");
