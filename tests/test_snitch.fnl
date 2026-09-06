(import-macros {: fn*} :howdah.snitch)
(local MiniTest (require :mini.test))
(local expect MiniTest.expect)

(local T (MiniTest.new_set))

(fn* add [a b]
  (+ a b))

(fn* documented [x]
  "Doubles x."
  (* x 2))

(fn* bind []
  (let [total 5
        {: row : col} {:row 1 :col 2}
        [first second] [:a :b]
        (q r) (values 7 8)]
    (+ total row col q r)))

(fn* mutate []
  (var n 0)
  (set n (+ n 1))
  (local m (* n 10))
  [n m])

(fn* nested []
  (let [outer 1]
    (let [inner (+ outer 1)]
      (+ outer inner))))

(fn* rest-args [head & tail]
  tail)

(fn* varargs [...]
  (select "#" ...))

(fn* whole []
  (let [[a &as all] [1 2]]
    all))

(tset T "captures parameters"
  (fn []
    (expect.equality (add 1 2) 3)
    (expect.equality _G.snitch {:a 1 :b 2})))

(tset T "exposes captures as bare globals"
  (fn []
    (add 4 5)
    (expect.equality _G.a 4)
    (expect.equality _G.b 5)))

(tset T "keeps a docstring"
  (fn []
    (expect.equality (documented 3) 6)
    (expect.equality _G.snitch {:x 3})))

(tset T "captures let bindings, destructured and multi-value included"
  (fn []
    (expect.equality (bind) 23)
    (expect.equality _G.snitch
                     {:total 5 :row 1 :col 2 :first :a :second :b :q 7 :r 8})))

(tset T "captures var, set and local"
  (fn []
    (expect.equality (mutate) [1 10])
    (expect.equality _G.snitch {:n 1 :m 10})))

(tset T "captures nested lets"
  (fn []
    (expect.equality (nested) 3)
    (expect.equality _G.snitch {:outer 1 :inner 2})))

(tset T "starts from an empty table on each call"
  (fn []
    (bind)
    (add 1 2)
    (expect.equality _G.snitch {:a 1 :b 2})))

(tset T "captures rest args and skips varargs"
  (fn []
    (expect.equality (rest-args 1 2 3) [2 3])
    (expect.equality _G.snitch {:head 1 :tail [2 3]})
    (expect.equality (varargs 1 2) 2)
    (expect.equality _G.snitch {})))

(tset T "captures &as bindings"
  (fn []
    (expect.equality (whole) [1 2])
    (expect.equality _G.snitch {:a 1 :all [1 2]})))

T
