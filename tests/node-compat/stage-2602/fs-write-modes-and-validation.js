"use strict";

const assert = require("assert");
const fs = require("fs");
const os = require("os");
const path = require("path");

process.umask(0o000);
const dir = fs.mkdtempSync(path.join(os.tmpdir(), "quench-fs-write-"));

// writeFileSync honors the { mode } option.
{
  const file = path.join(dir, "mode.txt");
  fs.writeFileSync(file, "123", { mode: 0o755 });
  assert.strictEqual(fs.statSync(file).mode & 0o777, 0o755);
  assert.strictEqual(fs.readFileSync(file, "utf8"), "123");
}

// appendFileSync honors { mode } for a new file.
{
  const file = path.join(dir, "append-mode.txt");
  fs.appendFileSync(file, "abc", { mode: 0o600 });
  assert.strictEqual(fs.statSync(file).mode & 0o777, 0o600);
  assert.strictEqual(fs.readFileSync(file, "utf8"), "abc");
}

// Non-string/Buffer data is rejected with ERR_INVALID_ARG_TYPE.
{
  const file = path.join(dir, "invalid.txt");
  for (const data of [true, 5, {}, [], null, undefined, Symbol()]) {
    assert.throws(
      () => fs.appendFileSync(file, data, { mode: 0o600 }),
      { code: "ERR_INVALID_ARG_TYPE" },
      `data ${typeof data}`
    );
  }
}

process.umask(0o022);

console.log("fs write data validation and modes passed");
