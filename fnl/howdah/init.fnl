(local howdah {})

(local render (require :howdah.render))
(local errors (require :howdah.errors))

(fn howdah-error [err]
  (error {:howdah-error err}))

(fn rpc [method ...]
  "Calls a server method. A server-side failure is re-raised as a table so
  interactive entrypoints can tell it apart from a plain Lua error (a bug)."
  (let [(ok result) (pcall vim.rpcrequest howdah.channel method ...)]
    (if ok result (howdah-error result))))

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
  ;; listed = true; scratch = false
  (let [buffer (vim.api.nvim_create_buf true false)]
    (vim.api.nvim_set_current_buf buffer)
    (set vim.bo.filetype :sql)
    ;; Set up keymaps
    (vim.keymap.set :n :<localleader>eb howdah.run
                    {: buffer :desc "Howdah: run buffer"})
    (vim.keymap.set :x :<localleader>E howdah.run-selection
                    {: buffer :desc "Howdah: run selection"})))

(fn howdah.query [sql]
  (rpc :query sql))

(fn run-sql [sql start]
  "Runs sql and renders the outcome. start is the zero-based row and byte
  column where sql begins in the current buffer."
  (let [buffer (vim.api.nvim_get_current_buf)]
    (errors.clear-diagnostic buffer)
    (case (howdah.query sql)
      {:Ok result} (render.show result)
      {:Err err} (do
                   (render.display (errors.format err sql start))
                   (errors.set-diagnostic buffer err sql start)))))

(fn howdah.run []
  (let [lines (vim.api.nvim_buf_get_lines 0 0 -1 false)]
    (run-sql (table.concat lines "\n") {:row 0 :byte-col 0})))

(fn howdah.run-selection []
  (let [start (vim.fn.getpos :v)
        end (vim.fn.getpos ".")
        opts {:type (vim.fn.mode)}
        lines (vim.fn.getregion start end opts)
        ;; getregionpos sorts the segments, so the first one starts the selection
        [[[_ line col]]] (vim.fn.getregionpos start end opts)]
    ;; The selection starts at (line, col), so line-1 rows and col-1 bytes
    ;; of the buffer precede it
    (run-sql (table.concat lines "\n") {:row (- line 1) :byte-col (- col 1)})))

(comment (howdah.start)
  (howdah.connect "host=localhost user=noahmoss dbname=howdah_dev")
  (render.show (howdah.query "select 1")))

howdah
