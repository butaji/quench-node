"use strict";

const assert = require("assert");
const path = require("path");

// win32.parse produces a stable {root,dir,base,ext,name} record and format
// round-trips it.
const winCases = [
  ["C:\\path\\dir\\index.html", "C:\\", "C:\\path\\dir", "index.html"],
  ["C:", "C:", "C:", ""],
  ["C:.", "C:", "C:", "."],
  ["C:abc", "C:", "C:", "abc"],
  ["C:\\abc", "C:\\", "C:\\", "abc"],
  ["\\", "\\", "\\", ""],
  ["file:stream", "", "", "file:stream"],
  [
    "\\\\server\\share\\file_path",
    "\\\\server\\share\\",
    "\\\\server\\share\\",
    "file_path"
  ]
];
for (const [input, root, dir, base] of winCases) {
  const parts = path.win32.parse(input);
  assert.strictEqual(parts.root, root, `win32 root ${input}`);
  assert.strictEqual(parts.dir, dir, `win32 dir ${input}`);
  assert.strictEqual(parts.base, base, `win32 base ${input}`);
  assert.strictEqual(
    path.win32.format(parts),
    input,
    `win32 roundtrip ${input}`
  );
  assert.strictEqual(path.win32.format(parts) === input, true);
}

// posix.parse treats backslash as an ordinary character.
const posCases = [
  ["/home/user/dir/file.txt", "/", "/home/user/dir", "file.txt"],
  ["/", "/", "/", ""],
  [".", "", "", "."],
  ["file", "", "", "file"],
  ["C:\\foo", "", "", "C:\\foo"],
  ["/foo.bar", "/", "/", "foo.bar"],
  ["/foo/bar.baz", "/", "/foo", "bar.baz"]
];
for (const [input, root, dir, base] of posCases) {
  const parts = path.posix.parse(input);
  assert.strictEqual(parts.root, root, `posix root ${input}`);
  assert.strictEqual(parts.dir, dir, `posix dir ${input}`);
  assert.strictEqual(parts.base, base, `posix base ${input}`);
  assert.strictEqual(
    path.posix.format(parts),
    input,
    `posix roundtrip ${input}`
  );
}

// format derives base from name + ext, and joins dir with the separator.
assert.strictEqual(
  path.posix.format({ dir: "some/dir", name: "index", ext: ".html" }),
  "some/dir/index.html"
);
assert.strictEqual(path.posix.format({ name: "x", ext: ".png" }), "x.png");
assert.strictEqual(path.posix.format({ name: "x", ext: "png" }), "x.png");
assert.strictEqual(
  path.win32.format({ root: "C:\\", name: "index", ext: ".html" }),
  "C:\\index.html"
);

console.log("path parse/format passed");
