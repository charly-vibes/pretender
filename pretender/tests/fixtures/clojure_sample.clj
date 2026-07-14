;; expected_complexity: simple=1, with-branch=2, complex-func=5
(defn simple [x]
  (inc x))

(defn with-branch [x]
  (if (pos? x)
    x
    (- x)))

(defn complex-func [a b items]
  (let [total (atom 0)]
    (if (pos? a)
      (if (pos? b)
        (swap! total + a b)))
    (doseq [item items]
      (if (even? item)
        (swap! total + item)))
    @total))