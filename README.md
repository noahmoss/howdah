# Howdah

*The best seat on the elephant.*

A Neovim plugin for querying, understanding, and administering a PostgreSQL
instance without leaving your editor. 

Status: early development, pre-alpha.

## Prerequisites

- Rust toolchain (`cargo`)
- Neovim (0.10+)
- A reachable PostgreSQL instance

## Install (development)

```
git clone <repo-url> ~/Projects/howdah
```

With [lazy.nvim](https://github.com/folke/lazy.nvim), add a spec pointing at the
checkout. The `build` step compiles the backend binary on install and update:

```lua
{
  dir = vim.fn.expand("~/Projects/howdah"),
  build = "cargo build",
}
```

lazy adds the repo to `runtimepath` and runs the build for you. To build by hand:

```
cargo build
```

## Usage

In Neovim:

```
:lua require("howdah").connect()   -- launch the backend, open the RPC channel
:lua =require("howdah").ping()     -- health check, returns "pong"
:lua require("howdah").run()       -- run the current buffer as SQL, render results
```

## Development

The frontend is written in Fennel and compiled to Lua by
[nfnl](https://github.com/Olical/nfnl). Editing `fnl/howdah/init.fnl` and saving
auto-writes `lua/howdah/init.lua` — commit both.

Because `require` caches modules, reload after a Fennel edit to pick it up in a
running session:

```
:lua package.loaded.howdah = nil
:lua require("howdah")
```

The Rust backend is a separate process. Rebuild and relaunch it (`cargo build`,
then `connect` again) to pick up Rust changes; Fennel edits never touch the
running server.
