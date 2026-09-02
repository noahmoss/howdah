local subcommands = {
	open = {
		impl = function(arg)
			require("howdah").open(arg)
		end,
	},
	start = {
		impl = function()
			require("howdah").start()
		end,
	},
	connect = {
		impl = function(arg)
			require("howdah").connect(arg)
		end,
	},
	stop = {
		impl = function()
			require("howdah").stop()
		end,
	},
}

vim.api.nvim_create_user_command("Howdah", function(opts)
	local verb, rest = opts.args:match("^(%S*)%s*(.*)$")
	if verb == "" then
		verb = "open"
	end
	local subcommand = subcommands[verb]
	if not subcommand then
		vim.notify("Howdah: unknown subcommand '" .. verb .. "'", vim.log.levels.ERROR)
		return
	end
	subcommand.impl(rest)
end, { nargs = "*" })
