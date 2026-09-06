-- [nfnl] tests/test_render.fnl
local MiniTest = require("mini.test")
local expect = MiniTest.expect
local render = require("howdah.render")
local T = MiniTest.new_set()
T["format-table"] = MiniTest.new_set()
local function _1_()
  return expect.equality(render["format-table"]({cols = {"a", "bb"}, rows = {{"1", "2"}}, row_count = 1}), {"a | bb", "--+---", "1 | 2 "})
end
T["format-table"]["renders a result set as an aligned table"] = _1_
local function _2_()
  return expect.equality(render["format-table"]({rows = {}, row_count = 2}), {})
end
T["format-table"]["renders nothing without a result set"] = _2_
T.summary = MiniTest.new_set()
local function _3_()
  expect.equality(render.summary({cols = {"a"}, rows = {{"1"}}, row_count = 1}), "1 row")
  return expect.equality(render.summary({cols = {"a"}, rows = {}, row_count = 0}), "0 rows")
end
T.summary["counts the rows of a result set"] = _3_
local function _4_()
  return expect.equality(render.summary({rows = {}, row_count = 2}), "2 rows affected")
end
T.summary["reports rows affected when there is no result set"] = _4_
local function _5_()
  return expect.equality(render.summary({rows = {}, row_count = 0}), "done")
end
T.summary["reads as done when nothing was returned or affected"] = _5_
T.status = MiniTest.new_set()
local function _6_()
  return expect.equality(render.status("1 row", {index = 1, total = 1}), "1 row")
end
T.status["is the summary alone for a single statement"] = _6_
local function _7_()
  return expect.equality(render.status("1 row", {index = 2, total = 3}), "1 row \194\183 statement 2 of 3")
end
T.status["places the statement in a multi-statement run"] = _7_
local function _8_()
  return expect.equality(render["format-table"]({cols = {"id"}, rows = {}, row_count = 0}), {"id", "--"})
end
T["format-table"]["keeps headers for an empty result set"] = _8_
local function _9_()
  return expect.equality(render["format-table"]({cols = {"a", "b"}, rows = {{"\231\149\140", "\195\169"}, {"x", "y"}}, row_count = 2}), {"a  | b", "---+--", "\231\149\140 | \195\169", "x  | y"})
end
T["format-table"]["pads cells by display width rather than byte length"] = _9_
return T
