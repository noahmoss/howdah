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

-- Completes the subcommand name only. Once a verb has been typed in full,
-- the rest of the line is that verb's own argument and we offer nothing.
local function complete_subcommand(arglead, cmdline)
	local ok, parsed = pcall(vim.api.nvim_parse_cmd, cmdline, {})
	if not ok then
		return {}
	end
	-- arglead is the word still being typed, so it doesn't count as finished.
	local finished_args = #parsed.args - (arglead ~= "" and 1 or 0)
	if finished_args > 0 then
		return {}
	end
	local matches = vim.tbl_filter(function(verb)
		return vim.startswith(verb, arglead)
	end, vim.tbl_keys(subcommands))
	table.sort(matches)
	return matches
end

vim.api.nvim_create_user_command("Howdah", function(opts)
	local verb, rest = opts.args:match("^(%S*)%s*(.*)$")
	if verb == "" then
		verb = "open"
	end
	local subcommand = subcommands[verb]
	if not subcommand then
		vim.notify("Howdah: unknown subcommand '" .. verb .. "' (try :Howdah <Tab>)", vim.log.levels.ERROR)
		return
	end
	-- Server-side failures are raised by the rpc helper as a tagged table. Show
	-- those as a notification; let anything else through unchanged, since that's
	-- a bug and the Lua traceback is wanted.
	local ok, err = pcall(subcommand.impl, rest)
	if not ok then
		if type(err) == "table" and err["howdah-error"] then
			-- The server formats error chains one cause per line. Fold that
			-- into a single line: anything taller than the command line
			-- triggers a Press-ENTER prompt.
			local message = err["howdah-error"]:gsub("\nCaused by: ", ": "):gsub("\n", " ")
			vim.notify("Howdah: " .. message, vim.log.levels.ERROR)
		else
			error(err, 0)
		end
	end
end, { nargs = "*", complete = complete_subcommand })
