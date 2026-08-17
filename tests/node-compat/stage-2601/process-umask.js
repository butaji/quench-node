"use strict";

const assert = require("assert");

const before = process.umask();
assert.strictEqual(typeof before, "number");

// Setting returns the previous value and takes effect for a bare read.
const previous = process.umask(0o077);
assert.strictEqual(previous, before);
assert.strictEqual(process.umask() & 0o777, 0o077);

// Octal-string masks are accepted too.
const back = process.umask("0o022");
assert.strictEqual(back & 0o777, 0o077);
assert.strictEqual(process.umask() & 0o777, 0o022);

// Restore the prior umask.
process.umask(before);
assert.strictEqual(process.umask() & 0o777, before & 0o777);

console.log("process.umask passed");
