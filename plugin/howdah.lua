vim.api.nvim_create_user_command("Howdah", function(opts)
	require("howdah").open(opts.args)
end, { nargs = "*" })
