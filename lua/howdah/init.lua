-- [nfnl] fnl/howdah/init.fnl
local howdah = {}
howdah.connect = function()
  howdah.channel = vim.fn.jobstart({"/Users/noahmoss/Projects/howdah/target/debug/howdah-server"}, {rpc = true})
  return nil
end
howdah.ping = function()
  return vim.rpcrequest(howdah.channel, "ping")
end
howdah.query = function(sql)
  return vim.rpcrequest(howdah.channel, "query", sql)
end
--[[ (howdah.connect) (howdah.query "select * from bird") ]]
return howdah
