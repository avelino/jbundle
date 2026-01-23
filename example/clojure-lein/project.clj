(defproject example/app "0.1.0"
  :description "Example Clojure app for jbundle"
  :dependencies [[org.clojure/clojure "1.12.0"]]
  :main example.core
  :aot [example.core]
  :profiles {:uberjar {:aot :all}})
