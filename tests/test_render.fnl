(local MiniTest (require :mini.test))
(local expect MiniTest.expect)
(local render (require :howdah.render))

(local T (MiniTest.new_set))

(set T.format-table (MiniTest.new_set))

(tset T.format-table "renders a result set as an aligned table"
  (fn []
    (expect.equality (render.format-table {:cols [:a :bb] :rows [[:1 :2]] :row_count 1})
                     ["a | bb" "--+---" "1 | 2 "])))

(tset T.format-table "renders nothing without a result set"
  (fn []
    (expect.equality (render.format-table {:rows [] :row_count 2}) [])))

(set T.summary (MiniTest.new_set))

(tset T.summary "counts the rows of a result set"
  (fn []
    (expect.equality (render.summary {:cols [:a] :rows [[:1]] :row_count 1})
                     "1 row")
    (expect.equality (render.summary {:cols [:a] :rows [] :row_count 0})
                     "0 rows")))

(tset T.summary "reports rows affected when there is no result set"
  (fn []
    (expect.equality (render.summary {:rows [] :row_count 2})
                     "2 rows affected")))

(tset T.summary "reads as done when nothing was returned or affected"
  (fn []
    (expect.equality (render.summary {:rows [] :row_count 0}) "done")))

(set T.status (MiniTest.new_set))

(tset T.status "is the summary alone for a single statement"
  (fn []
    (expect.equality (render.status "1 row" {:index 1 :total 1}) "1 row")))

(tset T.status "places the statement in a multi-statement run"
  (fn []
    (expect.equality (render.status "1 row" {:index 2 :total 3})
                     "1 row · statement 2 of 3")))

T
