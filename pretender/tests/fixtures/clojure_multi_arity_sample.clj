;; Multi-arity fixture (pretender-1v8k.5, slice 2.1).
;; expected_units: multi-arity x2 (params 1 and 2), private-multi x2 (params 1 and 2), single-arity x1
(defn multi-arity
  ([x]
   x)
  ([x y]
   (if (> x y)
     x y)))

(defn- private-multi
  ([a] (inc a))
  ([a b] (+ a b)))

(defn single-arity [n]
  (* n 2))
