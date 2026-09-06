-- [nfnl] tests/test_errors.fnl
local MiniTest = require("mini.test")
local expect = MiniTest.expect
local errors = require("howdah.errors")
local T = MiniTest.new_set()
T.format = MiniTest.new_set()
local function _1_()
  local err = {severity = "ERROR", code = "P0001", message = "Something went wrong"}
  return expect.equality(errors.format(err, "", {row = 0, ["byte-col"] = 0}), {"ERROR P0001: Something went wrong"})
end
T.format["formats a single-line message"] = _1_
local function _2_()
  local err = {severity = "ERROR", code = "P0001", message = "First line\nSecond line"}
  return expect.equality(errors.format(err, "", {row = 0, ["byte-col"] = 0}), {"ERROR P0001: First line", "Second line"})
end
T.format["splits a multiline message into physical lines"] = _2_
local function _3_()
  local err = {severity = "ERROR", code = "P0001", message = "failed", detail = "first\nsecond", hint = "try again", context = "function f"}
  return expect.equality(errors.format(err, "", {row = 0, ["byte-col"] = 0}), {"ERROR P0001: failed", "DETAIL: first", "second", "HINT: try again", "CONTEXT: function f"})
end
T.format["includes multiline detail, hint and context"] = _3_
local function _4_()
  local err = {severity = "ERROR", code = "42601", message = "bad syntax", position = 4}
  return expect.equality(errors.format(err, "ok\n\231\149\140x", {row = 4, ["byte-col"] = 7}), {"ERROR 42601: bad syntax", "LINE 6: \231\149\140x", "        ^"})
end
T.format["numbers source excerpts relative to the query buffer"] = _4_
local function _5_()
  local err = {severity = "ERROR", code = "42601", message = "bad syntax", position = 2}
  return expect.equality(errors.format(err, "\231\149\140x", {row = 0, ["byte-col"] = 0}), {"ERROR 42601: bad syntax", "LINE 1: \231\149\140x", "          ^"})
end
T.format["aligns the caret after a wide character"] = _5_
local function _6_()
  local err = {severity = "ERROR", code = "42601", message = "bad syntax", internal_query = "ok\n\231\149\140x", internal_position = 5}
  return expect.equality(errors.format(err, "select f()", {row = 8, ["byte-col"] = 3}), {"ERROR 42601: bad syntax", "QUERY: \231\149\140x", "         ^"})
end
T.format["positions an internal query independently of the source SQL"] = _6_
local function _7_()
  local err = {severity = "ERROR", code = "P0001", message = "failed", internal_query = "select 1"}
  return expect.equality(errors.format(err, "", {row = 0, ["byte-col"] = 0}), {"ERROR P0001: failed", "QUERY: select 1"})
end
T.format["includes an internal query without a position"] = _7_
local buffer = nil
local namespace = vim.api.nvim_create_namespace("howdah")
local other_namespace = vim.api.nvim_create_namespace("howdah-test-other")
local function _8_()
  buffer = vim.api.nvim_create_buf(false, true)
  return vim.api.nvim_buf_set_lines(buffer, 0, -1, false, {"prefix caf\195\169 x", "caf\195\169 x"})
end
local function _9_()
  return vim.api.nvim_buf_delete(buffer, {force = true})
end
T.diagnostics = MiniTest.new_set({hooks = {pre_case = _8_, post_case = _9_}})
local function _10_()
  errors["set-diagnostic"](buffer, {severity = "ERROR", code = "42601", message = "bad syntax", position = 6}, "caf\195\169 x", {row = 0, ["byte-col"] = 7})
  local _let_11_ = vim.diagnostic.get(buffer, {namespace = namespace})
  local diagnostic = _let_11_[1]
  expect.equality({diagnostic.lnum, diagnostic.col}, {0, 13})
  return expect.equality({diagnostic.message, diagnostic.code, diagnostic.source, diagnostic.severity}, {"bad syntax", "42601", "howdah", vim.diagnostic.severity.ERROR})
end
T.diagnostics["adds the selection byte offset on the first line"] = _10_
local function _12_()
  errors["set-diagnostic"](buffer, {severity = "ERROR", code = "42601", message = "bad syntax", position = 9}, "ok\ncaf\195\169 x", {row = 0, ["byte-col"] = 7})
  local _let_13_ = vim.diagnostic.get(buffer, {namespace = namespace})
  local diagnostic = _let_13_[1]
  return expect.equality({diagnostic.lnum, diagnostic.col}, {1, 6})
end
T.diagnostics["does not add the selection column on later lines"] = _12_
local function _14_()
  errors["set-diagnostic"](buffer, {severity = "ERROR", code = "P0001", message = "failed", internal_query = "select 1", internal_position = 1}, "select f()", {row = 0, ["byte-col"] = 0})
  return expect.equality(vim.diagnostic.get(buffer, {namespace = namespace}), {})
end
T.diagnostics["does not invent a source position for an internal error"] = _14_
local function _15_()
  vim.diagnostic.set(other_namespace, buffer, {{lnum = 0, col = 0, message = "another plugin"}})
  errors["set-diagnostic"](buffer, {severity = "ERROR", code = "42601", message = "bad syntax", position = 1}, "select", {row = 0, ["byte-col"] = 0})
  errors["clear-diagnostic"](buffer)
  expect.equality(vim.diagnostic.get(buffer, {namespace = namespace}), {})
  return expect.equality(#vim.diagnostic.get(buffer, {namespace = other_namespace}), 1)
end
T.diagnostics["clears only Howdah diagnostics"] = _15_
return T
