"use strict";

const assert = require("assert");
const os = require("os");

// os.EOL is the platform line separator.
assert.strictEqual(os.EOL, process.platform === "win32" ? "\r\n" : "\n");

// os.EOL is read-only but configurable, so it can be replaced via defineProperty.
const foo = "foo";
Object.defineProperties(os, {
  EOL: {
    configurable: true,
    enumerable: true,
    writable: false,
    value: foo
  }
});
assert.strictEqual(os.EOL, foo);

console.log("os EOL passed");
