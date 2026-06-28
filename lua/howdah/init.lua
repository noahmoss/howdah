local howdah = {}

function howdah.connect()
	howdah.channel = vim.fn.jobstart({
		"/Users/noahmoss/Projects/howdah/target/debug/howdah-server",
	}, { rpc = true })
end

function howdah.ping()
	return vim.rpcrequest(howdah.channel, "ping")
end

function howdah.query(sql)
	return vim.rpcrequest(howdah.channel, "query", sql)
end

return howdah
