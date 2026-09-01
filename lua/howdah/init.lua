-- [nfnl] fnl/howdah/init.fnl
local howdah = {}
local function rpc(method, ...)
  return vim.rpcrequest(howdah.channel, method, ...)
end
local function resolve_binary()
  local release_path = "target/release/howdah-server"
  local debug_path = "target/debug/howdah-server"
  local binary = (vim.api.nvim_get_runtime_file(release_path, false)[1] or vim.api.nvim_get_runtime_file(debug_path, false)[1])
  return assert(binary, "howdah: could not resolve server binary")
end
howdah.start = function()
  if not howdah.channel then
    local binary = resolve_binary()
    local channel = vim.fn.jobstart({binary}, {rpc = true})
    assert((channel ~= 0), "howdah: failed to start server: invalid arguments")
    assert((channel ~= -1), "howdah: failed to start server: not an executable")
    howdah.channel = channel
    return nil
  else
    return nil
  end
end
howdah.stop = function()
  if howdah.channel then
    vim.fn.jobstop(howdah.channel)
    howdah.channel = nil
    return nil
  else
    return nil
  end
end
local function not_empty(s)
  if (s ~= "") then
    return s
  else
    return nil
  end
end
local pg_vars = {{"PGHOST", "host"}, {"PGPORT", "port"}, {"PGUSER", "user"}, {"PGDATABASE", "dbname"}, {"PGPASSWORD", "password"}}
local function build_from_pg_vars()
  local kvs
  do
    local tbl_26_ = {}
    local i_27_ = 0
    for _i, _4_ in ipairs(pg_vars) do
      local env_var = _4_[1]
      local key = _4_[2]
      local val_28_
      do
        local val = not_empty(vim.env[env_var])
        if val then
          val_28_ = (key .. "=" .. val)
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
    kvs = tbl_26_
  end
  return table.concat(kvs, " ")
end
local function resolve_connection_string(arg)
  return (not_empty(arg) or not_empty(vim.env.DATABASE_URL) or build_from_pg_vars())
end
howdah.connect = function(target)
  if not howdah.channel then
    howdah.start()
  else
  end
  return rpc("connect", resolve_connection_string(target))
end
howdah.open = function(target)
  howdah.connect(target)
  local buffer = vim.api.nvim_create_buf(true, true)
  vim.api.nvim_set_current_buf(buffer)
  vim.bo.filetype = "sql"
  return nil
end
howdah.query = function(sql)
  return rpc("query", sql)
end
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
  local _8_
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
    _8_ = tbl_26_
  end
  return table.concat(_8_, " | ")
end
local function separator(widths)
  local _10_
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
    _10_ = tbl_26_
  end
  return table.concat(_10_, "-+-")
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
howdah.show = function(_12_)
  local cols = _12_.cols
  local rows = _12_.rows
  local lines = format_results(cols, rows)
  local buffer = vim.api.nvim_create_buf(false, true)
  vim.api.nvim_buf_set_lines(buffer, 0, -1, false, lines)
  return vim.api.nvim_open_win(buffer, true, {split = "below"})
end
howdah.run = function()
  local lines = vim.api.nvim_buf_get_lines(0, 0, -1, false)
  local sql = table.concat(lines, "\n")
  return howdah.show(howdah.query(sql))
end
--[[ (howdah.start) (howdah.connect "host=localhost user=noahmoss dbname=howdah_dev") (howdah.show (howdah.query "select 1")) ]]
return howdah
