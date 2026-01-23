pub fn generate(payload_hash: &str, payload_size: u64, jvm_args: &[String]) -> String {
    let jvm_args_str = if jvm_args.is_empty() {
        String::new()
    } else {
        format!(" {}", jvm_args.join(" "))
    };

    format!(
        r#"#!/bin/sh
set -e
CACHE_ID="{payload_hash}"
CACHE_DIR="${{HOME}}/.jbundle/cache/${{CACHE_ID}}"
PAYLOAD_SIZE={payload_size}

cat >&2 <<'BANNER'
   _ _                    _ _
  (_) |__  _   _ _ __   __| | | ___
  | | '_ \| | | | '_ \ / _` | |/ _ \
  | | |_) | |_| | | | | (_| | |  __/
 _/ |_.__/ \__,_|_| |_|\__,_|_|\___|
|__/
BANNER

if [ ! -d "$CACHE_DIR/runtime" ]; then
    mkdir -p "$CACHE_DIR"
    echo "Extracting runtime (first run)..." >&2
    tail -c "$PAYLOAD_SIZE" "$0" | tar xzf - -C "$CACHE_DIR"
fi

exec "$CACHE_DIR/runtime/bin/java"{jvm_args_str} -jar "$CACHE_DIR/app.jar" "$@"
exit 0
# --- PAYLOAD BELOW ---
"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stub_starts_with_shebang() {
        let stub = generate("abc123", 1024, &[]);
        assert!(stub.starts_with("#!/bin/sh\n"));
    }

    #[test]
    fn stub_contains_payload_hash() {
        let stub = generate("deadbeef12345678", 2048, &[]);
        assert!(stub.contains("CACHE_ID=\"deadbeef12345678\""));
    }

    #[test]
    fn stub_contains_payload_size() {
        let stub = generate("abc", 999999, &[]);
        assert!(stub.contains("PAYLOAD_SIZE=999999"));
    }

    #[test]
    fn stub_without_jvm_args() {
        let stub = generate("abc", 100, &[]);
        assert!(stub.contains("exec \"$CACHE_DIR/runtime/bin/java\" -jar"));
    }

    #[test]
    fn stub_with_jvm_args() {
        let args = vec!["-Xmx512m".to_string(), "-Dapp.env=prod".to_string()];
        let stub = generate("abc", 100, &args);
        assert!(stub.contains("exec \"$CACHE_DIR/runtime/bin/java\" -Xmx512m -Dapp.env=prod -jar"));
    }

    #[test]
    fn stub_ends_with_payload_marker() {
        let stub = generate("abc", 100, &[]);
        assert!(stub.ends_with("# --- PAYLOAD BELOW ---\n"));
    }

    #[test]
    fn stub_contains_banner() {
        let stub = generate("abc", 100, &[]);
        assert!(stub.contains("BANNER"));
        assert!(stub.contains("(_) |__"));
    }
}
