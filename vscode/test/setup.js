// Intercept `require('vscode')` to use our mock at runtime
const Module = require("module");
const path = require("path");

const originalResolve = Module._resolveFilename;
Module._resolveFilename = function (request, parent, isMain, options) {
  if (request === "vscode") {
    return path.join(__dirname, "..", "out-test", "test", "__mocks__", "vscode.js");
  }
  return originalResolve.call(this, request, parent, isMain, options);
};
