(local errors {})

(fn locate [text position]
  "Finds the line of text holding a Postgres position (1-based char offset).
  Returns {:line :col :text}: the line number, the count of chars on that line
  before the position, and the line's text."
  (let [chars-before (- position 1)
        prefix-lines (vim.split (vim.fn.strcharpart text 0 chars-before) "\n")
        line (length prefix-lines)]
    {: line
     :col (vim.fn.strchars (. prefix-lines line))
     :text (. (vim.split text "\n") line)}))

(fn labelled [label text]
  "psql's `LABEL: text` convention."
  (.. label ": " text))

(fn caret-under [{: text : col} label]
  "A line of spaces ending in a caret under col of the labelled text."
  (let [before-caret (labelled label (vim.fn.strcharpart text 0 col))]
    (.. (string.rep " " (vim.fn.strdisplaywidth before-caret)) "^")))

(fn headline [err]
  [(labelled (.. err.severity " " err.code) err.message)])

(fn detail-lines [err]
  "DETAIL, HINT and CONTEXT, whichever Postgres sent."
  (icollect [_ key (ipairs [:detail :hint :context])]
    (case (. err key) value (labelled (key:upper) value))))

(fn source-lines [err sql [rows-before _]]
  "The offending line of sql with a caret under the error, numbered as in the
  query buffer."
  (if err.position
      (let [location (locate sql err.position)
            buffer-line (+ rows-before location.line)
            label (.. "LINE " buffer-line)]
        [(labelled label location.text) (caret-under location label)])
      []))

(fn internal-query-lines [err]
  "The query Postgres ran on the user's behalf (e.g. inside a function), with a
  caret when it reported a position into it."
  (if (and err.internal_query err.internal_position)
      (let [location (locate err.internal_query err.internal_position)]
        [(labelled :QUERY location.text) (caret-under location :QUERY)])
      err.internal_query
      [(labelled :QUERY err.internal_query)]
      []))

(fn errors.format [err sql start]
  "Formats a SQL error as lines for the results buffer. start is how much of
  the query buffer precedes sql, as [rows bytes]."
  (-> (vim.iter [(headline err)
                 (detail-lines err)
                 (source-lines err sql start)
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

(fn buffer-position [sql position [rows-before bytes-before]]
  "Maps a Postgres position (1-based chars into sql) to the query buffer, as
  [line col]: the buffer line number and the count of bytes before it."
  (let [{: line : col : text} (locate sql position)
        byte (vim.str_byteindex text :utf-32 col)
        ;; Only the first line of sql can start mid-way through a buffer line
        bytes-before-line (if (= line 1) bytes-before 0)]
    [(+ rows-before line) (+ bytes-before-line byte)]))

(fn diagnostic [err sql start]
  "A vim.diagnostic entry for the error's position in the query buffer."
  (let [[line col] (buffer-position sql err.position start)
        ;; Diagnostics count lines from 0
        lnum (- line 1)]
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
