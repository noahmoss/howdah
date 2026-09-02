-- [nfnl] fnl/howdah/render.fnl
local render = {}
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
local function format_results(cols, rows)
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
end
render.show = function(_5_)
  local cols = _5_.cols
  local rows = _5_.rows
  local lines = format_results(cols, rows)
  local buffer = vim.api.nvim_create_buf(false, true)
  vim.api.nvim_buf_set_lines(buffer, 0, -1, false, lines)
  return vim.api.nvim_open_win(buffer, true, {split = "below"})
end
return render
