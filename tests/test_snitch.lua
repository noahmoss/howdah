-- [nfnl] tests/test_snitch.fnl
local MiniTest = require("mini.test")
local expect = MiniTest.expect
local T = MiniTest.new_set()
local function add(a, b)
  do
    _G["snitch"] = {}
    _G.snitch["a"] = a
    _G["a"] = a
    _G.snitch["b"] = b
    _G["b"] = b
  end
  return (a + b)
end
local function documented(x)
  do
    _G["snitch"] = {}
    _G.snitch["x"] = x
    _G["x"] = x
  end
  return (x * 2)
end
local function bind()
  do
    _G["snitch"] = {}
  end
  local total = 5
  local _let_1_ = {row = 1, col = 2}
  local row = _let_1_.row
  local col = _let_1_.col
  local first,second = "a", "b"
  local q, r = 7, 8
  do
    _G.snitch["total"] = total
    _G["total"] = total
    _G.snitch["row"] = row
    _G["row"] = row
    _G.snitch["col"] = col
    _G["col"] = col
    _G.snitch["first"] = first
    _G["first"] = first
    _G.snitch["second"] = second
    _G["second"] = second
    _G.snitch["q"] = q
    _G["q"] = q
    _G.snitch["r"] = r
    _G["r"] = r
  end
  return (total + row + col + q + r)
end
local function mutate()
  do
    _G["snitch"] = {}
  end
  local n = 0
  do
    _G.snitch["n"] = n
    _G["n"] = n
  end
  n = (n + 1)
  do
    _G.snitch["n"] = n
    _G["n"] = n
  end
  local m = (n * 10)
  do
    _G.snitch["m"] = m
    _G["m"] = m
  end
  return {n, m}
end
local function nested()
  do
    _G["snitch"] = {}
  end
  local outer = 1
  do
    _G.snitch["outer"] = outer
    _G["outer"] = outer
  end
  local inner = (outer + 1)
  do
    _G.snitch["inner"] = inner
    _G["inner"] = inner
  end
  return (outer + inner)
end
local function rest_args(head, ...)
  local tail = {...}
  do
    _G["snitch"] = {}
    _G.snitch["head"] = head
    _G["head"] = head
    _G.snitch["tail"] = tail
    _G["tail"] = tail
  end
  return tail
end
local function varargs(...)
  do
    _G["snitch"] = {}
  end
  return select("#", ...)
end
local function whole()
  do
    _G["snitch"] = {}
  end
  local _let_2_ = {1, 2}
  local a = _let_2_[1]
  local all = _let_2_
  do
    _G.snitch["a"] = a
    _G["a"] = a
    _G.snitch["all"] = all
    _G["all"] = all
  end
  return all
end
local function _3_()
  expect.equality(add(1, 2), 3)
  return expect.equality(_G.snitch, {a = 1, b = 2})
end
T["captures parameters"] = _3_
local function _4_()
  add(4, 5)
  expect.equality(_G.a, 4)
  return expect.equality(_G.b, 5)
end
T["exposes captures as bare globals"] = _4_
local function _5_()
  expect.equality(documented(3), 6)
  return expect.equality(_G.snitch, {x = 3})
end
T["keeps a docstring"] = _5_
local function _6_()
  expect.equality(bind(), 23)
  return expect.equality(_G.snitch, {total = 5, row = 1, col = 2, first = "a", second = "b", q = 7, r = 8})
end
T["captures let bindings, destructured and multi-value included"] = _6_
local function _7_()
  expect.equality(mutate(), {1, 10})
  return expect.equality(_G.snitch, {n = 1, m = 10})
end
T["captures var, set and local"] = _7_
local function _8_()
  expect.equality(nested(), 3)
  return expect.equality(_G.snitch, {outer = 1, inner = 2})
end
T["captures nested lets"] = _8_
local function _9_()
  bind()
  add(1, 2)
  return expect.equality(_G.snitch, {a = 1, b = 2})
end
T["starts from an empty table on each call"] = _9_
local function _10_()
  expect.equality(rest_args(1, 2, 3), {2, 3})
  expect.equality(_G.snitch, {head = 1, tail = {2, 3}})
  expect.equality(varargs(1, 2), 2)
  return expect.equality(_G.snitch, {})
end
T["captures rest args and skips varargs"] = _10_
local function _11_()
  expect.equality(whole(), {1, 2})
  return expect.equality(_G.snitch, {a = 1, all = {1, 2}})
end
T["captures &as bindings"] = _11_
return T
