"use strict";

const assert = require("assert");
const url = require("url");

assert.throws(() => url.parse(null), {
  code: "ERR_INVALID_ARG_TYPE",
  name: "TypeError"
});
assert.throws(() => url.parse(1), {
  code: "ERR_INVALID_ARG_TYPE",
  name: "TypeError"
});
assert.throws(() => url.parse({}), {
  code: "ERR_INVALID_ARG_TYPE",
  name: "TypeError"
});

assert.throws(
  () => {
    url.parse("http://%E0%A4%A@fail");
  },
  (error) => error instanceof URIError && error.code === undefined
);

assert.throws(() => url.parse("http://[127.0.0.1\x00c8763]:8000/"), {
  code: "ERR_INVALID_URL",
  input: "http://[127.0.0.1\x00c8763]:8000/"
});

assert.throws(() => url.parse("https://evil.com:.example.com"), {
  code: "ERR_INVALID_ARG_VALUE"
});
assert.throws(() => url.parse("git+ssh://git@github.com:npm/npm"), {
  code: "ERR_INVALID_ARG_VALUE"
});

console.log("url.parse invalid input passed");
