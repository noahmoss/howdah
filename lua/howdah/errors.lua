-- [nfnl] fnl/howdah/errors.fnl
local errors = {}
local function locate(text, position)
  local chars_before = (position - 1)
  local prefix_lines = vim.split(vim.fn.strcharpart(text, 0, chars_before), "\n")
  local line = #prefix_lines
  return {line = line, col = vim.fn.strchars(prefix_lines[line]), text = vim.split(text, "\n")[line]}
end
local function labelled(label, text)
  return (label .. ": " .. text)
end
local function caret_under(_1_, label)
  local text = _1_.text
  local col = _1_.col
  local before_caret = labelled(label, vim.fn.strcharpart(text, 0, col))
  return (string.rep(" ", vim.fn.strdisplaywidth(before_caret)) .. "^")
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
      local case_2_ = err[key]
      if (nil ~= case_2_) then
        local value = case_2_
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
local function source_lines(err, sql, _5_)
  local rows_before = _5_[1]
  local _ = _5_[2]
  if err.position then
    local location = locate(sql, err.position)
    local buffer_line = (rows_before + location.line)
    local label = ("LINE " .. buffer_line)
    return {labelled(label, location.text), caret_under(location, label)}
  else
    return {}
  end
end
local function internal_query_lines(err)
  if (err.internal_query and err.internal_position) then
    local location = locate(err.internal_query, err.internal_position)
    return {labelled("QUERY", location.text), caret_under(location, "QUERY")}
  elseif err.internal_query then
    return {labelled("QUERY", err.internal_query)}
  else
    return {}
  end
end
errors.format = function(err, sql, start)
  return vim.iter({headline(err), detail_lines(err), source_lines(err, sql, start), internal_query_lines(err)}):flatten():totable()
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
local function buffer_position(sql, position, _9_)
  local rows_before = _9_[1]
  local bytes_before = _9_[2]
  local _let_10_ = locate(sql, position)
  local line = _let_10_.line
  local col = _let_10_.col
  local text = _let_10_.text
  local byte = vim.str_byteindex(text, "utf-32", col)
  local bytes_before_line
  if (line == 1) then
    bytes_before_line = bytes_before
  else
    bytes_before_line = 0
  end
  return {(rows_before + line), (bytes_before_line + byte)}
end
local function diagnostic(err, sql, start)
  local _let_12_ = buffer_position(sql, err.position, start)
  local line = _let_12_[1]
  local col = _let_12_[2]
  local lnum = (line - 1)
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
