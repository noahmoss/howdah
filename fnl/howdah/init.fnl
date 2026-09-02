(local howdah {})

(local render (require :howdah.render))

(fn rpc [method ...]
  (vim.rpcrequest howdah.channel method ...))

(fn resolve-binary []
  (let [release-path :target/release/howdah-server
        debug-path :target/debug/howdah-server
        binary (or (. (vim.api.nvim_get_runtime_file release-path false) 1)
                   (. (vim.api.nvim_get_runtime_file debug-path false) 1))]
    (assert binary "howdah: could not resolve server binary")))

(fn howdah.start []
  "Starts the Howdah backend process and initializes the channel for msgpack RPCs."
  (when (not howdah.channel)
    (let [binary (resolve-binary)
          channel (vim.fn.jobstart [binary] {:rpc true})]
      (assert (not= channel 0)
              "howdah: failed to start server: invalid arguments")
      (assert (not= channel -1)
              "howdah: failed to start server: not an executable")
      (set howdah.channel channel))))

(fn howdah.stop []
  "Stops the running server, if one exists, and clears the RPC channel."
  (when howdah.channel
    (vim.fn.jobstop howdah.channel)
    (set howdah.channel nil)))

(fn not-empty [s]
  "Returns s if it is not an empty string; otherwise nil."
  (when (not= s "") s))

(local pg-vars [[:PGHOST :host]
                [:PGPORT :port]
                [:PGUSER :user]
                [:PGDATABASE :dbname]
                [:PGPASSWORD :password]])

(fn build-from-pg-vars []
  "Builds a string of key-value connection parameters from standard PostgreSQL
  environment variables, if set."
  (let [kvs (icollect [_i [env-var key] (ipairs pg-vars)]
              (let [val (not-empty (. vim.env env-var))]
                (when val (.. key "=" val))))]
    (table.concat kvs " ")))

(fn resolve-connection-string [arg]
  (or (not-empty arg) (not-empty vim.env.DATABASE_URL) (build-from-pg-vars)))

(fn howdah.connect [target]
  "Initializes a connection to a PostgreSQL instance, using a passed-in
  connection string or environment variables. Starts the server first if an RPC
  channel has not been set up."
  (when (not howdah.channel)
    (howdah.start))
  (rpc :connect (resolve-connection-string target)))

(fn howdah.open [target]
  "Starts a new Howdah session, initializing a new connection and opening a SQL
  buffer."
  (howdah.connect target)
  (let [buffer (vim.api.nvim_create_buf true true)]
    (vim.api.nvim_set_current_buf buffer)
    (set vim.bo.filetype :sql)
    ;; Set up keymaps
    (vim.keymap.set :n :<localleader>eb howdah.run
                    {: buffer :desc "Howdah: run buffer"})
    (vim.keymap.set :x :<localleader>E howdah.run-selection
                    {: buffer :desc "Howdah: run selection"})))

(fn howdah.query [sql]
  (rpc :query sql))

(fn howdah.run []
  (let [lines (vim.api.nvim_buf_get_lines 0 0 -1 false)
        sql (table.concat lines "\n")]
    (render.show (howdah.query sql))))

(fn howdah.run-selection []
  (let [lines (vim.fn.getregion (vim.fn.getpos :v) (vim.fn.getpos ".")
                                {:type (vim.fn.mode)})
        sql (table.concat lines "\n")]
    (render.show (howdah.query sql))))

(comment (howdah.start)
  (howdah.connect "host=localhost user=noahmoss dbname=howdah_dev")
  (render.show (howdah.query "select 1")))

howdah
