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
return T
