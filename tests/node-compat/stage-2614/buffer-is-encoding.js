"use strict";

const assert = require("assert");

for (const enc of [
  "hex",
  "utf8",
  "utf-8",
  "ascii",
  "latin1",
  "binary",
  "base64",
  "base64url",
  "ucs2",
  "ucs-2",
  "utf16le",
  "utf-16le"
]) {
  assert.strictEqual(Buffer.isEncoding(enc), true);
}

for (const enc of ["utf9", "utf-7", false, NaN, {}, Infinity, [], 1, 0, -1]) {
  assert.strictEqual(Buffer.isEncoding(enc), false);
}

console.log("Buffer.isEncoding passed");
