-- [nfnl] fnl/howdah/init.fnl
local howdah = {}
local render = require("howdah.render")
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
howdah.run = function()
  local lines = vim.api.nvim_buf_get_lines(0, 0, -1, false)
  local sql = table.concat(lines, "\n")
  return render.show(howdah.query(sql))
end
--[[ (howdah.start) (howdah.connect "host=localhost user=noahmoss dbname=howdah_dev") (render.show (howdah.query "select 1")) ]]
return howdah
