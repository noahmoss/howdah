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

(tset T.format "includes multiline detail, hint and context"
  (fn []
    (let [err {:severity :ERROR :code :P0001 :message "failed"
               :detail "first\nsecond" :hint "try again" :context "function f"}]
      (expect.equality (errors.format err "" {:row 0 :byte-col 0})
                       ["ERROR P0001: failed" "DETAIL: first" "second"
                        "HINT: try again" "CONTEXT: function f"]))))

(tset T.format "numbers source excerpts relative to the query buffer"
  (fn []
    (let [err {:severity :ERROR :code :42601 :message "bad syntax" :position 4}]
      (expect.equality (errors.format err "ok\n界x" {:row 4 :byte-col 7})
                       ["ERROR 42601: bad syntax" "LINE 6: 界x" "        ^"]))))

(tset T.format "aligns the caret after a wide character"
  (fn []
    (let [err {:severity :ERROR :code :42601 :message "bad syntax" :position 2}]
      (expect.equality (errors.format err "界x" {:row 0 :byte-col 0})
                       ["ERROR 42601: bad syntax" "LINE 1: 界x" "          ^"]))))

(tset T.format "positions an internal query independently of the source SQL"
  (fn []
    (let [err {:severity :ERROR :code :42601 :message "bad syntax"
               :internal_query "ok\n界x" :internal_position 5}]
      (expect.equality (errors.format err "select f()" {:row 8 :byte-col 3})
                       ["ERROR 42601: bad syntax" "QUERY: 界x" "         ^"]))))

(tset T.format "includes an internal query without a position"
  (fn []
    (let [err {:severity :ERROR :code :P0001 :message "failed"
               :internal_query "select 1"}]
      (expect.equality (errors.format err "" {:row 0 :byte-col 0})
                       ["ERROR P0001: failed" "QUERY: select 1"]))))

(var buffer nil)
(local namespace (vim.api.nvim_create_namespace :howdah))
(local other-namespace (vim.api.nvim_create_namespace :howdah-test-other))

(set T.diagnostics
  (MiniTest.new_set
    {:hooks
     {:pre_case (fn []
                  (set buffer (vim.api.nvim_create_buf false true))
                  (vim.api.nvim_buf_set_lines buffer 0 -1 false
                                              ["prefix café x" "café x"]))
      :post_case (fn []
                   (vim.api.nvim_buf_delete buffer {:force true}))}}))

(tset T.diagnostics "adds the selection byte offset on the first line"
  (fn []
    (errors.set-diagnostic buffer
                          {:severity :ERROR :code :42601 :message "bad syntax" :position 6}
                          "café x" {:row 0 :byte-col 7})
    (let [[diagnostic] (vim.diagnostic.get buffer {:namespace namespace})]
      (expect.equality [diagnostic.lnum diagnostic.col] [0 13])
      (expect.equality [diagnostic.message diagnostic.code diagnostic.source diagnostic.severity]
                       ["bad syntax" :42601 :howdah vim.diagnostic.severity.ERROR]))))

(tset T.diagnostics "does not add the selection column on later lines"
  (fn []
    (errors.set-diagnostic buffer
                          {:severity :ERROR :code :42601 :message "bad syntax" :position 9}
                          "ok\ncafé x" {:row 0 :byte-col 7})
    (let [[diagnostic] (vim.diagnostic.get buffer {:namespace namespace})]
      (expect.equality [diagnostic.lnum diagnostic.col] [1 6]))))

(tset T.diagnostics "does not invent a source position for an internal error"
  (fn []
    (errors.set-diagnostic buffer
                          {:severity :ERROR :code :P0001 :message "failed"
                           :internal_query "select 1" :internal_position 1}
                          "select f()" {:row 0 :byte-col 0})
    (expect.equality (vim.diagnostic.get buffer {:namespace namespace}) [])))

(tset T.diagnostics "clears only Howdah diagnostics"
  (fn []
    (vim.diagnostic.set other-namespace buffer
                        [{:lnum 0 :col 0 :message "another plugin"}])
    (errors.set-diagnostic buffer
                          {:severity :ERROR :code :42601 :message "bad syntax" :position 1}
                          "select" {:row 0 :byte-col 0})
    (errors.clear-diagnostic buffer)
    (expect.equality (vim.diagnostic.get buffer {:namespace namespace}) [])
    (expect.equality (length (vim.diagnostic.get buffer {:namespace other-namespace})) 1)))

T
