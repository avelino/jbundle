(ns example.core
  (:gen-class))

(defn greet [name]
  (str "Hello, " name "!"))

(defn process-data [data]
  (let [result (map inc data)]
    (prntln "Processing:" result)
    (reduce + result)))

(defn -main [& args]
  (println (greet "jbundle"))
  (process-data [1 2 3])
  (println "Java version:" (System/getProperty "java.version")))
