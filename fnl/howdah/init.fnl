(local howdah {})

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

(fn howdah.connect [connection-string]
  "Initializes a connection to a PostgreSQL instance, given a connection
  string. Starts the server first if an RPC channel has not been set up."
  (when (not howdah.channel)
    (howdah.start))
  (rpc :connect connection-string))

(fn howdah.query [sql]
  (rpc :query sql))

(fn cell-width [s] (vim.fn.strdisplaywidth s))

(fn compute-widths [cols rows]
  "Returns a table of per-column display widths."
  (local widths {})
  (for [i 1 (length cols)]
    (set (. widths i) (cell-width (. cols i))))
  (each [_ row (ipairs rows)]
    (for [i 1 (length row)]
      (set (. widths i) (math.max (. widths i) (cell-width (. row i))))))
  widths)

(fn format-row [row widths]
  (table.concat (icollect [i cell (ipairs row)]
                  (.. cell (string.rep " " (- (. widths i) (cell-width cell)))))
                " | "))

(fn separator [widths]
  (table.concat (icollect [_ w (ipairs widths)]
                  (string.rep "-" w)) "-+-"))

(fn format-results [cols rows]
  (let [widths (compute-widths cols rows)
        header (format-row cols widths)
        sep (separator widths)
        lines [header sep]]
    (icollect [_ row (ipairs rows) &into lines]
      (format-row row widths))))

(fn howdah.show [{: cols : rows}]
  (let [lines (format-results cols rows)
        buffer (vim.api.nvim_create_buf false true)]
    (vim.api.nvim_buf_set_lines buffer 0 -1 false lines)
    (vim.api.nvim_open_win buffer true {:split :below})))

(fn howdah.run []
  (let [lines (vim.api.nvim_buf_get_lines 0 0 -1 false)
        sql (table.concat lines "\n")]
    (howdah.show (howdah.query sql))))

(comment (howdah.start)
  (howdah.connect "host=localhost user=noahmoss dbname=howdah_dev")
  (howdah.show (howdah.query "select 1")))

howdah
