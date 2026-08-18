"use strict";

const assert = require("assert");
const fs = require("fs");

// fstat must reject a non-number fd with ERR_INVALID_ARG_TYPE (not an
// internal NotCallable), including symbols the engine stores as strings.
for (const input of ["", false, null, undefined, {}, [], Symbol("x")]) {
  assert.throws(
    () => fs.fstat(input),
    { code: "ERR_INVALID_ARG_TYPE", name: "TypeError" },
    `fstat ${String(input)}`
  );
}

console.log("fs fstat validation passed");
