"use strict";

const assert = require("assert");
const qs = require("querystring");

// escape() coerces primitives to strings and percent-encodes.
assert.strictEqual(qs.escape(5), "5");
assert.strictEqual(qs.escape("test"), "test");
assert.strictEqual(qs.escape({}), "%5Bobject%20Object%5D");
assert.strictEqual(qs.escape([5, 10]), "5%2C10");
assert.strictEqual(qs.escape("Ŋōđĕ"), "%C5%8A%C5%8D%C4%91%C4%95");
assert.strictEqual(qs.escape("testŊōđĕ"), "test%C5%8A%C5%8D%C4%91%C4%95");

// escape() honours a callable toString() and falls back to valueOf().
assert.strictEqual(
  qs.escape({ test: 5, toString: () => "test", valueOf: () => 10 }),
  "test"
);
assert.strictEqual(qs.escape({ toString: 5, valueOf: () => "test" }), "test");

console.log("querystring escape passed");
