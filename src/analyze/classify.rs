#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntryCategory {
    Class,
    Resource,
    NativeLib,
    Metadata,
    ClojureSource,
    JavaSource,
}

pub fn classify_entry(name: &str) -> EntryCategory {
    if name.ends_with(".class") {
        return EntryCategory::Class;
    }
    if name.ends_with(".clj") || name.ends_with(".cljc") || name.ends_with(".cljs") {
        return EntryCategory::ClojureSource;
    }
    if name.ends_with(".java") {
        return EntryCategory::JavaSource;
    }
    if name.ends_with(".so")
        || name.ends_with(".dylib")
        || name.ends_with(".dll")
        || name.ends_with(".jnilib")
    {
        return EntryCategory::NativeLib;
    }
    if name.starts_with("META-INF/") {
        return EntryCategory::Metadata;
    }
    EntryCategory::Resource
}

pub fn extract_package(name: &str) -> String {
    let parts: Vec<&str> = name.split('/').collect();
    let depth = parts.len().saturating_sub(1).min(3);
    if depth == 0 {
        return "(root)".to_string();
    }
    parts[..depth].join(".")
}

pub fn detect_clojure_ns(name: &str) -> Option<String> {
    let stem = name.strip_suffix(".class")?;
    let base = stem.strip_suffix("__init")?;
    Some(base.replace('/', "."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_class_files() {
        assert_eq!(
            classify_entry("com/example/Main.class"),
            EntryCategory::Class
        );
        assert_eq!(
            classify_entry("clojure/core__init.class"),
            EntryCategory::Class
        );
    }

    #[test]
    fn classify_clojure_sources() {
        assert_eq!(
            classify_entry("clojure/core.clj"),
            EntryCategory::ClojureSource
        );
        assert_eq!(
            classify_entry("myapp/core.cljc"),
            EntryCategory::ClojureSource
        );
        assert_eq!(
            classify_entry("myapp/main.cljs"),
            EntryCategory::ClojureSource
        );
    }

    #[test]
    fn classify_java_sources() {
        assert_eq!(
            classify_entry("com/example/Main.java"),
            EntryCategory::JavaSource
        );
    }

    #[test]
    fn classify_native_libs() {
        assert_eq!(
            classify_entry("lib/native/libfoo.so"),
            EntryCategory::NativeLib
        );
        assert_eq!(
            classify_entry("lib/native/libfoo.dylib"),
            EntryCategory::NativeLib
        );
        assert_eq!(classify_entry("foo.dll"), EntryCategory::NativeLib);
        assert_eq!(
            classify_entry("lib/native/libfoo.jnilib"),
            EntryCategory::NativeLib
        );
    }

    #[test]
    fn classify_metadata() {
        assert_eq!(
            classify_entry("META-INF/MANIFEST.MF"),
            EntryCategory::Metadata
        );
        assert_eq!(
            classify_entry("META-INF/maven/pom.xml"),
            EntryCategory::Metadata
        );
    }

    #[test]
    fn classify_resources() {
        assert_eq!(classify_entry("config.edn"), EntryCategory::Resource);
        assert_eq!(classify_entry("logback.xml"), EntryCategory::Resource);
        assert_eq!(
            classify_entry("templates/index.html"),
            EntryCategory::Resource
        );
    }

    #[test]
    fn extract_package_three_segments() {
        assert_eq!(
            extract_package("org/apache/commons/lang3/StringUtils.class"),
            "org.apache.commons"
        );
    }

    #[test]
    fn extract_package_fewer_segments() {
        assert_eq!(extract_package("com/Main.class"), "com");
        assert_eq!(extract_package("com/example/Main.class"), "com.example");
    }

    #[test]
    fn extract_package_root_file() {
        assert_eq!(extract_package("Main.class"), "(root)");
    }

    #[test]
    fn detect_clojure_ns_init_class() {
        assert_eq!(
            detect_clojure_ns("myapp/core__init.class"),
            Some("myapp.core".to_string())
        );
        assert_eq!(
            detect_clojure_ns("clojure/core__init.class"),
            Some("clojure.core".to_string())
        );
    }

    #[test]
    fn detect_clojure_ns_nested() {
        assert_eq!(
            detect_clojure_ns("myapp/handlers/api__init.class"),
            Some("myapp.handlers.api".to_string())
        );
    }

    #[test]
    fn detect_clojure_ns_not_init() {
        assert_eq!(detect_clojure_ns("myapp/core.class"), None);
        assert_eq!(detect_clojure_ns("myapp/core$fn__123.class"), None);
    }

    #[test]
    fn detect_clojure_ns_not_class() {
        assert_eq!(detect_clojure_ns("myapp/core__init.clj"), None);
    }
}
