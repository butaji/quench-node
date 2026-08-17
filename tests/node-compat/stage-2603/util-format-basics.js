"use strict";

const assert = require("assert");
const util = require("util");

const symbol = Symbol("foo");

// util.format() with a non-template argument inspects the value.
assert.strictEqual(util.format(), "");
assert.strictEqual(util.format([]), "[]");
assert.strictEqual(util.format([0]), "[ 0 ]");
assert.strictEqual(util.format({}), "{}");
assert.strictEqual(util.format({ foo: 42 }), "{ foo: 42 }");
assert.strictEqual(util.format(null), "null");
assert.strictEqual(util.format(true), "true");
assert.strictEqual(util.format("test"), "test");
assert.strictEqual(util.format("foo", "bar", "baz"), "foo bar baz");
assert.strictEqual(util.format(symbol), "Symbol(foo)");
assert.strictEqual(util.format("%s", symbol), "Symbol(foo)");
assert.strictEqual(util.format("%j", symbol), "undefined");

// %d/%i/%f number formatting including edge values (non -0 cases, which are
// engine-sign-flaky).
assert.strictEqual(util.format("%d", 1.5), "1.5");
assert.strictEqual(util.format("%d", Infinity), "Infinity");
assert.strictEqual(util.format("%d", -Infinity), "-Infinity");
assert.strictEqual(util.format("%d", Symbol()), "NaN");
assert.strictEqual(util.format("%i", 1.5), "1");
assert.strictEqual(util.format("%i", Symbol()), "NaN");
assert.strictEqual(util.format("%f", 1.5), "1.5");

console.log("util.format basics passed");
