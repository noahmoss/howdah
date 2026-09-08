(local render {})
(local errors (require :howdah.errors))

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

(fn render.format-table [{: cols : rows}]
  "Formats a result set as a table."
  (if cols
      (let [widths (compute-widths cols rows)
            header (format-row cols widths)
            sep (separator widths)
            lines [header sep]]
        (icollect [_ row (ipairs rows) &into lines]
          (format-row row widths)))
      []))

(fn plural [n word]
  (.. n " " word (if (= n 1) "" "s")))

(local command-verbs {:INSERT :inserted :UPDATE :updated :DELETE :deleted})

(fn render.summary [{: tag : row_count}]
  "Formats common command tags, falling back to the original tag."
  (let [command (string.match tag "^(%S+)")
        verb (. command-verbs command)]
    (if (and row_count (= command :SELECT)) (plural row_count "row")
        (and row_count verb) (.. (plural row_count "row") " " verb)
        tag)))

(fn render.status [summary {: index : total}]
  "The results window's status: the summary, placed in its run when the run
  had more than one statement."
  (if (> total 1)
      (.. summary " · statement " index " of " total)
      summary))

(var results-buffer nil)

(fn get-or-create-results-buffer []
  (when (or (not results-buffer)
            (not (vim.api.nvim_buf_is_valid results-buffer)))
    (set results-buffer (vim.api.nvim_create_buf false true)))
  results-buffer)

(fn render.display [lines status]
  "Replaces the results buffer contents with lines and puts status on its
  window's statusline, opening the split if needed."
  (let [buffer (get-or-create-results-buffer)]
    (vim.api.nvim_buf_set_lines buffer 0 -1 false lines)
    (let [window (vim.fn.bufwinid buffer)
          window (if (= window -1)
                     (vim.api.nvim_open_win buffer false {:split :below})
                     window)]
      ;; Window-local, so it wins over a plugin's global statusline
      (vim.api.nvim_set_option_value :statusline (.. " howdah results%=" status " ")
                                     {:win window}))))

(fn render.show [result statement-position sql sql-start]
  "Shows a statement's result or error with its status. sql and sql-start provide
  source context for errors. statement-position holds the index and run total."
  (case result
    {:ok query-result}
    (render.display (render.format-table query-result)
                    (render.status (render.summary query-result) statement-position))
    {:err err} (render.display (errors.format err sql sql-start)
                              (render.status "error" statement-position))))

render
