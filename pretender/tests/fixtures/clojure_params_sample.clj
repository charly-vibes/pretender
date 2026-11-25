;; Parameter-shape fixture (pretender-1v8k.6, proposal task 2.2).
;; expected_params: variadic=a,rest ; deep-bind=a,[b c] ; destr-map=x,{:keys [a b]}
(defn variadic [a & rest]
  (count rest))

(defn deep-bind [a [b c]]
  (+ a b c))

(defn destr-map [x {:keys [a b]}]
  (+ x a))
