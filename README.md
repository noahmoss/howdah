# Howdah

*The best seat on the elephant.*

A Neovim plugin for querying, understanding, and administering a PostgreSQL
instance without leaving your editor.

Status: early development, pre-alpha.

## Prerequisites

- Rust toolchain (`cargo`)
- Neovim (0.11+)
- A reachable PostgreSQL instance

## Install (development)

```
git clone https://github.com/noahmoss/Howdah.git ~/Projects/howdah
```

Add the checkout to [lazy.nvim](https://github.com/folke/lazy.nvim):

```lua
{
  dir = vim.fn.expand("~/Projects/howdah"),
  build = "cargo build",
}
```

## Usage

Open a SQL buffer and connect to a database:

```vim
:Howdah open host=localhost dbname=mydb user=myuser
```

Or use `:Howdah` to connect using `DATABASE_URL`, falling back to `PGHOST`,
`PGPORT`, `PGUSER`, `PGDATABASE`, and `PGPASSWORD`.

In a buffer opened by Howdah:

- `<localleader>eb` runs the whole buffer.
- `<localleader>E` in visual mode runs the selection.

Results and SQL errors appear in a split.

## Development

[nfnl](https://github.com/Olical/nfnl) compiles Fennel on save once you trust
`.nfnl.fnl`. Source lives in `fnl/howdah/`, compiled Lua in `lua/howdah/`.
Tests compile in place in `tests/`. Commit both Fennel and Lua.

Tests, from the repository:

- Rust: `cargo test --workspace`
- Fennel: `:lua MiniTest.run()` with [mini.test](https://github.com/nvim-mini/mini.test)
  set up and Howdah on your runtimepath.
