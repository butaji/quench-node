"use strict";

const assert = require("assert");
const util = require("util");

assert.strictEqual(util.format("%j", { a: 1 }), '{"a":1}');
assert.strictEqual(util.format("%j", [1, 2]), "[1,2]");
assert.strictEqual(util.format("%j", null), "null");
assert.strictEqual(util.format("%j", undefined), "undefined");
assert.strictEqual(util.format("%j", "hi"), '"hi"');
assert.strictEqual(util.format("%j", true), "true");

console.log("util.format json passed");
