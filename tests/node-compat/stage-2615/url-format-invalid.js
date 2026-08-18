"use strict";

const assert = require("assert");
const url = require("url");

for (const value of [
  undefined,
  null,
  true,
  false,
  0,
  function () {},
  Symbol("foo")
]) {
  assert.throws(() => url.format(value), {
    code: "ERR_INVALID_ARG_TYPE",
    name: "TypeError"
  });
}

assert.strictEqual(url.format(""), "");
assert.strictEqual(url.format({}), "");

console.log("url.format invalid input passed");
