"use strict";

const assert = require("assert");
const path = require("path");

assert.match("hello world", /world/);
assert.match("../../../../../x", /^(\.\.\/){3,5}x$/);
assert.doesNotMatch("hello", /world/);

const relativePath = path.posix.relative("a/b/c", "../../x");
assert.match(relativePath, /^(\.\.\/){3,5}x$/);

console.log("assert.match regex passed");
