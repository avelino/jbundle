(ns example.core
  (:gen-class))

(defn -main [& args]
  (println "Hello from clj-pack!")
  (println "Arguments:" (vec args))
  (println "Java version:" (System/getProperty "java.version"))
  (println "OS:" (System/getProperty "os.name") (System/getProperty "os.arch")))
