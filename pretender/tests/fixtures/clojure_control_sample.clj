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
