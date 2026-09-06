(local MiniTest (require :mini.test))
(local expect MiniTest.expect)
(local errors (require :howdah.errors))

(local T (MiniTest.new_set))

(set T.format (MiniTest.new_set))

(tset T.format "formats a single-line message"
  (fn []
    (let [err {:severity :ERROR
               :code :P0001
               :message "Something went wrong"}]
      (expect.equality (errors.format err "" {:row 0 :byte-col 0})
                       ["ERROR P0001: Something went wrong"]))))

(tset T.format "splits a multiline message into physical lines"
  (fn []
    (let [err {:severity :ERROR
               :code :P0001
               :message "First line\nSecond line"}]
      (expect.equality (errors.format err "" {:row 0 :byte-col 0})
                       ["ERROR P0001: First line" "Second line"]))))

T
