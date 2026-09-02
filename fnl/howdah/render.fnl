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

(fn render.show [{: cols : rows}]
  (let [lines (format-results cols rows)
        buffer (vim.api.nvim_create_buf false true)]
    (vim.api.nvim_buf_set_lines buffer 0 -1 false lines)
    (vim.api.nvim_open_win buffer true {:split :below})))

render
