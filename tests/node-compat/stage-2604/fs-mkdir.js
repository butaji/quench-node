"use strict";

const assert = require("assert");
const fs = require("fs");
const os = require("os");
const path = require("path");

const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "quench-mkdir-"));

// recursive mkdir returns the first directory created, or undefined if none.
{
  const a = path.join(tmp, "a");
  const b = path.join(a, "b");
  assert.strictEqual(
    fs.mkdirSync(b, { recursive: true }),
    path.toNamespacedPath(a)
  );
  assert.strictEqual(fs.mkdirSync(b, { recursive: true }), undefined);
  assert.strictEqual(fs.statSync(b).isDirectory(), true);
}

// mode option is honored.
{
  const m = path.join(tmp, "mode");
  assert.strictEqual(fs.mkdirSync(m, { mode: 0o755 }), undefined);
  assert.strictEqual(fs.statSync(m).mode & 0o777, 0o755);
}

// EEXIST for a recursive mkdir whose path is an existing file.
{
  const f = path.join(tmp, "sub", "file");
  fs.mkdirSync(path.dirname(f));
  fs.writeFileSync(f, "");
  assert.throws(() => fs.mkdirSync(f, { recursive: true }), {
    code: "EEXIST",
    syscall: "mkdir",
    name: "Error",
    path: f
  });
}

// recursive must be a boolean; anything else is rejected.
for (const recursive of ["", 1, {}, [], null, Symbol("x"), () => {}]) {
  assert.throws(() => fs.mkdirSync(path.join(tmp, "bad"), { recursive }), {
    code: "ERR_INVALID_ARG_TYPE",
    name: "TypeError"
  });
}

// async mkdir calls the callback with null on success and err on failure.
{
  const p = path.join(tmp, "async-ok");
  fs.mkdir(p, { mode: 0o755 }, (err) => {
    assert.strictEqual(err, null);
    assert.strictEqual(fs.statSync(p).mode & 0o777, 0o755);
  });
  const f = path.join(tmp, "sub-file2", "file");
  fs.mkdirSync(path.dirname(f));
  fs.writeFileSync(f, "");
  fs.mkdir(f, { recursive: true }, (err) => {
    assert.strictEqual(err.code, "EEXIST");
    assert.strictEqual(err.syscall, "mkdir");
  });
}

console.log("fs mkdir passed");
