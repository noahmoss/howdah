(local errors {})

(fn locate [text position]
  "Finds the line of text holding a Postgres position (1-based char offset).
  Returns the zero-based row and character column, plus the line's text."
  (let [chars-before (- position 1)
        lines (vim.split text "\n")
        prefix-lines (vim.split (vim.fn.strcharpart text 0 chars-before) "\n")
        line-index (length prefix-lines)]
    {:row (- line-index 1)
     :char-col (vim.fn.strchars (. prefix-lines line-index))
     :text (. lines line-index)}))

(fn labelled [label text]
  "psql's `LABEL: text` convention."
  (.. label ": " text))

(fn caret-under [{: text : char-col} label]
  "A line of spaces ending in a caret under char-col of the labelled text."
  (let [before-caret (labelled label (vim.fn.strcharpart text 0 char-col))]
    (.. (string.rep " " (vim.fn.strdisplaywidth before-caret)) "^")))

(fn positioned-lines [label location]
  "The positioned line of text and a caret under the reported character."
  [(labelled label location.text) (caret-under location label)])

(fn headline [err]
  [(labelled (.. err.severity " " err.code) err.message)])

(fn detail-lines [err]
  "DETAIL, HINT and CONTEXT, whichever Postgres sent."
  (icollect [_ key (ipairs [:detail :hint :context])]
    (let [value (. err key)]
      (when value
        (labelled (key:upper) value)))))

(fn buffer-query-lines [err sql start]
  "The offending line of sql with a caret under the error, numbered as in the
  query buffer."
  (if err.position
      (let [location (locate sql err.position)
            buffer-line (+ start.row location.row 1)
            label (.. "LINE " buffer-line)]
        (positioned-lines label location))
      []))

(fn internal-query-lines [err]
  "The query Postgres ran on the user's behalf (e.g. inside a function), with a
  caret when it reported a position into it."
  (if (and err.internal_query err.internal_position)
      (positioned-lines :QUERY
                        (locate err.internal_query err.internal_position))
      err.internal_query
      [(labelled :QUERY err.internal_query)]
      []))

(fn errors.format [err sql start]
  "Formats a SQL error as lines for the results buffer. start is the zero-based
  row and byte column where sql begins in the query buffer."
  (-> (vim.iter [(headline err)
                 (detail-lines err)
                 (buffer-query-lines err sql start)
                 (internal-query-lines err)])
      (: :flatten)
      (: :totable)))

;; Create a namespace so that we can clear only Howdah-related diagnostics
(local diagnostics-ns (vim.api.nvim_create_namespace :howdah))

(fn diagnostic-severity [severity]
  (case severity
    (where (or :ERROR :FATAL :PANIC)) vim.diagnostic.severity.ERROR
    :WARNING vim.diagnostic.severity.WARN
    _ vim.diagnostic.severity.INFO))

(fn buffer-position [sql position start]
  "Maps a Postgres position (1-based chars into sql) to the zero-based lnum and
  byte col expected by vim.diagnostic."
  (let [{: row : char-col : text} (locate sql position)
        byte-col (vim.str_byteindex text :utf-32 char-col)
        ;; Only the first line of sql can start mid-way through a buffer line
        start-byte-col (if (= row 0) start.byte-col 0)]
    {:lnum (+ start.row row) :col (+ start-byte-col byte-col)}))

(fn diagnostic [err sql start]
  "A vim.diagnostic entry for the error's position in the query buffer."
  (let [{: lnum : col} (buffer-position sql err.position start)]
    {: lnum
     : col
     :message err.message
     :severity (diagnostic-severity err.severity)
     :code err.code
     :source :howdah}))

(fn errors.set-diagnostic [buffer err sql start]
  "Marks the error position on the query buffer, if the error has one."
  (when err.position
    (vim.diagnostic.set diagnostics-ns buffer [(diagnostic err sql start)])))

(fn errors.clear-diagnostic [buffer]
  (vim.diagnostic.reset diagnostics-ns buffer))

errors
