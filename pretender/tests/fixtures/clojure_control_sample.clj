;; Control-form fixture (pretender-1v8k.9, proposal tasks 1.1/1.2/2.3).
;; expected_cyclomatic: condy=4, caser=3, condpr=3, looper=4, catcher=2, g=2
;; expected_abc: condy=3.606, caser=2.0, condpr=2.236, looper=3.606, catcher=1.414, g=3.162
(defn condy [a]
  (cond
    (pos? a) 1
    (neg? a) 2
    :else 3))

(defn caser [e]
  (case e
    a 1
    b 2
    3))

(defn condpr [x]
  (condp = x
    1 :one
    2 :two
    :other))

(defn looper [xs]
  (loop [xs xs]
    (if (seq xs)
      (recur (rest xs))
      xs)))

(defn catcher [f]
  (try
    (f)
    (catch Exception _ 1)
    (finally 2)))

(defn g [a]
  (if (pos? a)
    (let [b (inc a)]
      (println b))))

;; expected_cyclomatic: 4, expected_nesting: 3, expected_cognitive: 6
;; doseq -> when -> if: each branch-bearing body wraps one level deeper
;; (proposal D4); mirrors the audit's Python-deep row (4/6/3).
(defn deeply-nested [acc xss]
  (doseq [xs xss]
    (when (seq xs)
      (if (pos? (count xs))
        (swap! acc + (count xs))
        (reset! acc 0)))))

;; expected_nesting: 1 — a single conditional body nests once, like a
;; Python if-block.
(defn branchy [acc a]
  (if (pos? a)
    (swap! acc inc)
    (reset! acc 0)))

;; Deftest fixture (pretender-1v8k.8, task 2.5): deftest forms are Test
;; units even though they carry no parameter vector; is-family forms are
;; assertions and `testing` stays consumed scaffolding.
(deftest arith-test
  (is (= 4 (+ 2 2))))

(deftest throws-test
  (testing "context"
    (is (= 1 1))
    (are [x y] (= x y) 1 1)
    (thrown-with-msg? Exception #"boom" (throw (ex-info "boom" {})))))
