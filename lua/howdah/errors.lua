-- [nfnl] fnl/howdah/errors.fnl
local errors = {}
local function locate(text, position)
  local chars_before = (position - 1)
  local lines = vim.split(text, "\n")
  local prefix_lines = vim.split(vim.fn.strcharpart(text, 0, chars_before), "\n")
  local line_index = #prefix_lines
  return {row = (line_index - 1), ["char-col"] = vim.fn.strchars(prefix_lines[line_index]), text = lines[line_index]}
end
local function labelled(label, text)
  return (label .. ": " .. text)
end
local function caret_under(_1_, label)
  local text = _1_.text
  local char_col = _1_["char-col"]
  local before_caret = labelled(label, vim.fn.strcharpart(text, 0, char_col))
  return (string.rep(" ", vim.fn.strdisplaywidth(before_caret)) .. "^")
end
local function positioned_lines(label, location)
  return {labelled(label, location.text), caret_under(location, label)}
end
local function headline(err)
  return {labelled((err.severity .. " " .. err.code), err.message)}
end
local function detail_lines(err)
  local tbl_26_ = {}
  local i_27_ = 0
  for _, key in ipairs({"detail", "hint", "context"}) do
    local val_28_
    do
      local value = err[key]
      if value then
        val_28_ = labelled(key:upper(), value)
      else
        val_28_ = nil
      end
    end
    if (nil ~= val_28_) then
      i_27_ = (i_27_ + 1)
      tbl_26_[i_27_] = val_28_
    else
    end
  end
  return tbl_26_
end
local function buffer_query_lines(err, sql, start)
  if err.position then
    local location = locate(sql, err.position)
    local buffer_line = (start.row + location.row + 1)
    local label = ("LINE " .. buffer_line)
    return positioned_lines(label, location)
  else
    return {}
  end
end
local function internal_query_lines(err)
  if (err.internal_query and err.internal_position) then
    return positioned_lines("QUERY", locate(err.internal_query, err.internal_position))
  elseif err.internal_query then
    return {labelled("QUERY", err.internal_query)}
  else
    return {}
  end
end
errors.format = function(err, sql, start)
  return vim.iter({headline(err), detail_lines(err), buffer_query_lines(err, sql, start), internal_query_lines(err)}):flatten():totable()
end
local diagnostics_ns = vim.api.nvim_create_namespace("howdah")
local function diagnostic_severity(severity)
  if ((severity == "ERROR") or (severity == "FATAL") or (severity == "PANIC")) then
    return vim.diagnostic.severity.ERROR
  elseif (severity == "WARNING") then
    return vim.diagnostic.severity.WARN
  else
    local _ = severity
    return vim.diagnostic.severity.INFO
  end
end
local function buffer_position(sql, position, start)
  local _let_7_ = locate(sql, position)
  local row = _let_7_.row
  local char_col = _let_7_["char-col"]
  local text = _let_7_.text
  local byte_col = vim.str_byteindex(text, "utf-32", char_col)
  local start_byte_col
  if (row == 0) then
    start_byte_col = start["byte-col"]
  else
    start_byte_col = 0
  end
  return {lnum = (start.row + row), col = (start_byte_col + byte_col)}
end
local function diagnostic(err, sql, start)
  local _let_9_ = buffer_position(sql, err.position, start)
  local lnum = _let_9_.lnum
  local col = _let_9_.col
  return {lnum = lnum, col = col, message = err.message, severity = diagnostic_severity(err.severity), code = err.code, source = "howdah"}
end
errors["set-diagnostic"] = function(buffer, err, sql, start)
  if err.position then
    return vim.diagnostic.set(diagnostics_ns, buffer, {diagnostic(err, sql, start)})
  else
    return nil
  end
end
errors["clear-diagnostic"] = function(buffer)
  return vim.diagnostic.reset(diagnostics_ns, buffer)
end
return errors
