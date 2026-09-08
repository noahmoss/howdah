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
    (expect.equality (render.summary {:tag "SELECT 1" :row_count 1})
                     "1 row")
    (expect.equality (render.summary {:tag "SELECT 0" :row_count 0})
                     "0 rows")))

(tset T.summary "names the operation and handles singular and zero counts"
  (fn []
    (expect.equality (render.summary {:tag "INSERT 0 2" :row_count 2})
                     "2 rows inserted")
    (expect.equality (render.summary {:tag "UPDATE 0" :row_count 0})
                     "0 rows updated")
    (expect.equality (render.summary {:tag "DELETE 1" :row_count 1})
                     "1 row deleted")))

(tset T.summary "preserves tags without counts and unrecognized commands"
  (fn []
    (expect.equality (render.summary {:tag "CREATE TABLE"}) "CREATE TABLE")
    (expect.equality (render.summary {:tag "BEGIN"}) "BEGIN")
    (expect.equality (render.summary {:tag "COPY 2" :row_count 2}) "COPY 2")
    (expect.equality (render.summary {:tag "SELECT"}) "SELECT")))

(tset T.summary "keeps the operation for DML with returning rows"
  (fn []
    (expect.equality (render.summary {:tag "INSERT 0 1" :row_count 1
                                      :cols [:id] :rows [[:1]]})
                     "1 row inserted")))

(set T.status (MiniTest.new_set))

(tset T.status "is the summary alone for a single statement"
  (fn []
    (expect.equality (render.status "1 row" {:index 1 :total 1}) "1 row")))

(tset T.status "places the statement in a multi-statement run"
  (fn []
    (expect.equality (render.status "1 row" {:index 2 :total 3})
                     "1 row · statement 2 of 3")))

(tset T.format-table "keeps headers for an empty result set"
  (fn []
    (expect.equality (render.format-table {:cols [:id] :rows [] :row_count 0})
                     ["id" "--"])))

(tset T.format-table "pads cells by display width rather than byte length"
  (fn []
    (expect.equality (render.format-table {:cols [:a :b]
                                          :rows [["界" "é"] ["x" "y"]]
                                          :row_count 2})
                     ["a  | b" "---+--" "界 | é" "x  | y"])))

T
