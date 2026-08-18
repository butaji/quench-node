"use strict";

const assert = require("assert");
const path = require("path");

for (const impl of [path.posix, path.win32, path]) {
  for (const input of [null, {}, true, 1, undefined]) {
    assert.throws(() => impl.parse(input), {
      code: "ERR_INVALID_ARG_TYPE",
      name: "TypeError"
    });
  }
  for (const input of [null, undefined, "", true, false, 1]) {
    assert.throws(() => impl.format(input), {
      code: "ERR_INVALID_ARG_TYPE",
      name: "TypeError"
    });
  }
  assert.throws(
    () => impl.format("string"),
    (error) =>
      error.code === "ERR_INVALID_ARG_TYPE" &&
      error.name === "TypeError" &&
      String(error.message).includes("pathObject") &&
      String(error.message).includes("string")
  );
}

const parsed = path.posix.parse("/home/user/dir/file.txt");
assert.strictEqual(parsed.root, "/");
assert.strictEqual(parsed.dir, "/home/user/dir");
assert.strictEqual(parsed.base, "file.txt");
assert.strictEqual(parsed.ext, ".txt");
assert.strictEqual(parsed.name, "file");
assert.strictEqual(path.posix.format(parsed), "/home/user/dir/file.txt");

const trailing = path.posix.parse("./");
assert.strictEqual(trailing.dir, "");
assert.strictEqual(trailing.base, ".");
assert.strictEqual(trailing.name, ".");

console.log("path parse/format errors passed");
