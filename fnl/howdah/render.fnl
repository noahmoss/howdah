(local render {})

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

(var results-buffer nil)

(fn get-or-create-results-buffer []
  (when (or (not results-buffer)
            (not (vim.api.nvim_buf_is_valid results-buffer)))
    (set results-buffer (vim.api.nvim_create_buf false true)))
  results-buffer)

(fn render.display [lines]
  "Replaces the results buffer contents with lines, opening its split if needed."
  (let [buffer (get-or-create-results-buffer)]
    (vim.api.nvim_buf_set_lines buffer 0 -1 false lines)
    (when (= -1 (vim.fn.bufwinid buffer))
      (vim.api.nvim_open_win buffer false {:split :below}))))

(fn render.show [{: cols : rows}]
  (render.display (format-results cols rows)))

render
