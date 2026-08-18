"use strict";

const assert = require("assert");
const os = require("os");

assert.strictEqual(os.EOL, "\n");
assert.throws(() => {
  os.EOL = 123;
}, TypeError);

Object.defineProperties(os, {
  EOL: {
    configurable: true,
    enumerable: true,
    writable: false,
    value: "foo"
  }
});
assert.strictEqual(os.EOL, "foo");

console.log("os.EOL readonly passed");
