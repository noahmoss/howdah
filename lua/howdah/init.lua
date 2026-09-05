-- [nfnl] fnl/howdah/init.fnl
local howdah = {}
local render = require("howdah.render")
local errors = require("howdah.errors")
local function howdah_error(err)
  return error({["howdah-error"] = err})
end
local function rpc(method, ...)
  local ok, result = pcall(vim.rpcrequest, howdah.channel, method, ...)
  if ok then
    return result
  else
    return howdah_error(result)
  end
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
    for _i, _5_ in ipairs(pg_vars) do
      local env_var = _5_[1]
      local key = _5_[2]
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
  local buffer = vim.api.nvim_create_buf(true, false)
  vim.api.nvim_set_current_buf(buffer)
  vim.bo.filetype = "sql"
  vim.keymap.set("n", "<localleader>eb", howdah.run, {buffer = buffer, desc = "Howdah: run buffer"})
  return vim.keymap.set("x", "<localleader>E", howdah["run-selection"], {buffer = buffer, desc = "Howdah: run selection"})
end
howdah.query = function(sql)
  return rpc("query", sql)
end
local function run_sql(sql, start)
  local buffer = vim.api.nvim_get_current_buf()
  errors["clear-diagnostic"](buffer)
  local case_9_ = howdah.query(sql)
  if ((_G.type(case_9_) == "table") and (nil ~= case_9_.Ok)) then
    local result = case_9_.Ok
    return render.show(result)
  elseif ((_G.type(case_9_) == "table") and (nil ~= case_9_.Err)) then
    local err = case_9_.Err
    render.display(errors.format(err, sql, start))
    return errors["set-diagnostic"](buffer, err, sql, start)
  else
    return nil
  end
end
howdah.run = function()
  local lines = vim.api.nvim_buf_get_lines(0, 0, -1, false)
  return run_sql(table.concat(lines, "\n"), {0, 0})
end
howdah["run-selection"] = function()
  local start = vim.fn.getpos("v")
  local _end = vim.fn.getpos(".")
  local opts = {type = vim.fn.mode()}
  local lines = vim.fn.getregion(start, _end, opts)
  local _let_11_ = vim.fn.getregionpos(start, _end, opts)
  local _let_12_ = _let_11_[1]
  local _let_13_ = _let_12_[1]
  local _ = _let_13_[1]
  local line = _let_13_[2]
  local col = _let_13_[3]
  return run_sql(table.concat(lines, "\n"), {(line - 1), (col - 1)})
end
--[[ (howdah.start) (howdah.connect "host=localhost user=noahmoss dbname=howdah_dev") (render.show (howdah.query "select 1")) ]]
return howdah
