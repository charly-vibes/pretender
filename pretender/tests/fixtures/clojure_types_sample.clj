;; Zero-capture contract fixture (pretender-1v8k.12): the pinned grammar
;; (tree-sitter-clojure 0.1.0) exposes only generic list_lit/sym_lit nodes,
;; so defrecord/deftype ship with NO type captures; parsing stays valid
;; (spec: "Adapter without type captures stays valid").
(defrecord Point [x y])

(deftype Cell [held]
  (proto)
  (area [this] 1))
