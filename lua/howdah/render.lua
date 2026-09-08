-- [nfnl] fnl/howdah/render.fnl
local render = {}
local errors = require("howdah.errors")
local function cell_width(s)
  return vim.fn.strdisplaywidth(s)
end
local function compute_widths(cols, rows)
  local widths = {}
  for i = 1, #cols do
    widths[i] = cell_width(cols[i])
  end
  for _, row in ipairs(rows) do
    for i = 1, #row do
      widths[i] = math.max(widths[i], cell_width(row[i]))
    end
  end
  return widths
end
local function format_row(row, widths)
  local _1_
  do
    local tbl_26_ = {}
    local i_27_ = 0
    for i, cell in ipairs(row) do
      local val_28_ = (cell .. string.rep(" ", (widths[i] - cell_width(cell))))
      if (nil ~= val_28_) then
        i_27_ = (i_27_ + 1)
        tbl_26_[i_27_] = val_28_
      else
      end
    end
    _1_ = tbl_26_
  end
  return table.concat(_1_, " | ")
end
local function separator(widths)
  local _3_
  do
    local tbl_26_ = {}
    local i_27_ = 0
    for _, w in ipairs(widths) do
      local val_28_ = string.rep("-", w)
      if (nil ~= val_28_) then
        i_27_ = (i_27_ + 1)
        tbl_26_[i_27_] = val_28_
      else
      end
    end
    _3_ = tbl_26_
  end
  return table.concat(_3_, "-+-")
end
render["format-table"] = function(_5_)
  local cols = _5_.cols
  local rows = _5_.rows
  if cols then
    local widths = compute_widths(cols, rows)
    local header = format_row(cols, widths)
    local sep = separator(widths)
    local lines = {header, sep}
    local tbl_24_ = lines
    for _, row in ipairs(rows) do
      local val_25_ = format_row(row, widths)
      table.insert(tbl_24_, val_25_)
    end
    return tbl_24_
  else
    return {}
  end
end
local function plural(n, word)
  local _7_
  if (n == 1) then
    _7_ = ""
  else
    _7_ = "s"
  end
  return (n .. " " .. word .. _7_)
end
local command_verbs = {INSERT = "inserted", UPDATE = "updated", DELETE = "deleted"}
render.summary = function(_9_)
  local tag = _9_.tag
  local row_count = _9_.row_count
  local command = string.match(tag, "^(%S+)")
  local verb = command_verbs[command]
  if (row_count and (command == "SELECT")) then
    return plural(row_count, "row")
  elseif (row_count and verb) then
    return (plural(row_count, "row") .. " " .. verb)
  else
    return tag
  end
end
render.status = function(summary, _11_)
  local index = _11_.index
  local total = _11_.total
  if (total > 1) then
    return (summary .. " \194\183 statement " .. index .. " of " .. total)
  else
    return summary
  end
end
local results_buffer = nil
local function get_or_create_results_buffer()
  if (not results_buffer or not vim.api.nvim_buf_is_valid(results_buffer)) then
    results_buffer = vim.api.nvim_create_buf(false, true)
  else
  end
  return results_buffer
end
render.display = function(lines, status)
  local buffer = get_or_create_results_buffer()
  vim.api.nvim_buf_set_lines(buffer, 0, -1, false, lines)
  local window = vim.fn.bufwinid(buffer)
  local window0
  if (window == -1) then
    window0 = vim.api.nvim_open_win(buffer, false, {split = "below"})
  else
    window0 = window
  end
  return vim.api.nvim_set_option_value("statusline", (" howdah results%=" .. status .. " "), {win = window0})
end
render.show = function(result, statement_position, sql, sql_start)
  if ((_G.type(result) == "table") and (nil ~= result.ok)) then
    local query_result = result.ok
    return render.display(render["format-table"](query_result), render.status(render.summary(query_result), statement_position))
  elseif ((_G.type(result) == "table") and (nil ~= result.err)) then
    local err = result.err
    return render.display(errors.format(err, sql, sql_start), render.status("error", statement_position))
  else
    return nil
  end
end
return render
