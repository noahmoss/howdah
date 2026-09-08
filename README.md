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

- Rust: `make test` (or `cargo test --workspace`)
- Fennel: `:lua MiniTest.run()` with [mini.test](https://github.com/nvim-mini/mini.test)
  set up and Howdah on your runtimepath.

### PostgreSQL integration tests

Database tests are ignored by default. With Docker and Docker Compose installed
and the Docker daemon running, run them from the repository:

```sh
make test-integration
```

This starts PostgreSQL 18 on `127.0.0.1:55432`, waits for it to be ready, runs the
database tests, then removes the container and its data, including when tests fail.
Each test uses its own connection and temporary tables.

To use an existing test database instead:

```sh
HOWDAH_TEST_DATABASE_URL='your connection string' \
  cargo test --workspace -- --ignored
```
