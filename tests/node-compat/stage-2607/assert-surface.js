"use strict";

const assert = require("assert");

assert.ok(1);
assert.strictEqual(1, 1);
assert.notStrictEqual(1, 2);
assert.equal("1", 1);
assert.notEqual("1", 2);
assert.deepEqual({ a: 1 }, { a: "1" });
assert.notDeepEqual({ a: 1 }, { a: 2 });
assert.deepStrictEqual({ a: 1 }, { a: 1 });
assert.notDeepStrictEqual({ a: 1 }, { a: "1" });
assert.partialDeepStrictEqual({ a: 1, b: 2 }, { a: 1 });
assert.match("hello world", /world/);
assert.doesNotMatch("hello", /world/);

assert.throws(() => {
  throw new Error("boom");
}, /boom/);
assert.doesNotThrow(() => 1);
assert.ifError(null);
assert.ifError(undefined);

try {
  assert.fail("explicit");
  throw new Error("fail should throw");
} catch (error) {
  assert.strictEqual(error.name, "AssertionError");
  assert.strictEqual(error.code, "ERR_ASSERTION");
  assert.strictEqual(error.operator, "fail");
  assert.strictEqual(error.message, "explicit");
}

const constructed = assert.AssertionError("constructed");
assert.strictEqual(constructed.name, "AssertionError");
assert.strictEqual(constructed.code, "ERR_ASSERTION");

console.log("assert surface passed");
