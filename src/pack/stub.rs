use crate::config::JvmProfile;

pub struct StubParams<'a> {
    pub runtime_hash: &'a str,
    pub runtime_size: u64,
    pub app_hash: &'a str,
    pub app_size: u64,
    pub cds_hash: Option<&'a str>,
    pub cds_size: u64,
    pub crac_hash: Option<&'a str>,
    pub crac_size: u64,
    pub profile: &'a JvmProfile,
    pub jvm_args: &'a [String],
}

pub fn generate(params: &StubParams) -> String {
    let profile_flags = params.profile.flags().join(" ");
    let jvm_args_str = if params.jvm_args.is_empty() {
        String::new()
    } else {
        format!(" {}", params.jvm_args.join(" "))
    };

    let profile_and_args = if profile_flags.is_empty() {
        jvm_args_str.clone()
    } else if jvm_args_str.is_empty() {
        format!(" {profile_flags}")
    } else {
        format!(" {profile_flags}{jvm_args_str}")
    };

    let cds_hash_val = params.cds_hash.unwrap_or("");
    let crac_hash_val = params.crac_hash.unwrap_or("");

    let runtime_hash = params.runtime_hash;
    let runtime_size = params.runtime_size;
    let app_hash = params.app_hash;
    let app_size = params.app_size;
    let cds_size = params.cds_size;
    let crac_size = params.crac_size;

    format!(
        r#"#!/bin/sh
set -e
CACHE="${{HOME}}/.jbundle/cache"
RT_HASH="{runtime_hash}"    RT_SIZE={runtime_size}
APP_HASH="{app_hash}"   APP_SIZE={app_size}
CDS_SIZE={cds_size}        CDS_HASH="{cds_hash_val}"
CRAC_SIZE={crac_size}       CRAC_HASH="{crac_hash_val}"

cat >&2 <<'BANNER'
   _ _                    _ _
  (_) |__  _   _ _ __   __| | | ___
  | | '_ \| | | | '_ \ / _` | |/ _ \
  | | |_) | |_| | | | | (_| | |  __/
 _/ |_.__/ \__,_|_| |_|\__,_|_|\___|
|__/
BANNER

STUB_SIZE=$(( $(head -c 99999 "$0" 2>/dev/null | grep -c '' || wc -c < "$0") ))
STUB_SIZE=__STUB_SIZE__

# Extract runtime (only if not cached)
RT_DIR="$CACHE/rt-$RT_HASH"
if [ ! -d "$RT_DIR/bin" ]; then
    mkdir -p "$RT_DIR"
    echo "Extracting runtime (first run)..." >&2
    tail -c +$((STUB_SIZE + 1)) "$0" | head -c "$RT_SIZE" | tar xzf - -C "$RT_DIR"
fi

# Extract app.jar (only if not cached)
APP_DIR="$CACHE/app-$APP_HASH"
if [ ! -f "$APP_DIR/app.jar" ]; then
    mkdir -p "$APP_DIR"
    tail -c +$((STUB_SIZE + RT_SIZE + 1)) "$0" | head -c "$APP_SIZE" > "$APP_DIR/app.jar"
fi

# Extract AppCDS archive (if present and not cached)
CDS_FLAG=""
if [ "$CDS_SIZE" -gt 0 ] 2>/dev/null; then
    CDS_DIR="$CACHE/cds-$CDS_HASH"
    if [ ! -f "$CDS_DIR/app.jsa" ]; then
        mkdir -p "$CDS_DIR"
        tail -c +$((STUB_SIZE + RT_SIZE + APP_SIZE + 1)) "$0" | head -c "$CDS_SIZE" > "$CDS_DIR/app.jsa"
    fi
    CDS_FLAG="-XX:SharedArchiveFile=$CDS_DIR/app.jsa"
fi

# CRaC restore (Linux only)
if [ "$CRAC_SIZE" -gt 0 ] 2>/dev/null && [ "$(uname)" = "Linux" ]; then
    CRAC_DIR="$CACHE/crac-$CRAC_HASH"
    if [ ! -d "$CRAC_DIR/cr" ]; then
        mkdir -p "$CRAC_DIR"
        tail -c +$((STUB_SIZE + RT_SIZE + APP_SIZE + CDS_SIZE + 1)) "$0" | head -c "$CRAC_SIZE" | tar xzf - -C "$CRAC_DIR"
    fi
    exec "$RT_DIR/bin/java" -XX:CRaCRestoreFrom="$CRAC_DIR/cr" "$@" 2>/dev/null || true
fi

# Launch with profile flags + AppCDS + user args
exec "$RT_DIR/bin/java"{profile_and_args} $CDS_FLAG -jar "$APP_DIR/app.jar" "$@"
exit 0
# --- PAYLOAD BELOW ---
"#
    )
}

/// Replace the __STUB_SIZE__ placeholder with the actual byte size of the stub
pub fn finalize_stub(stub: &str) -> String {
    let placeholder = "__STUB_SIZE__";
    // Calculate what the final size will be:
    // The stub length after replacing placeholder with the actual number
    // We need to iterate since the number of digits affects the total size
    let base_len = stub.len() - placeholder.len();
    let mut size = base_len + 1; // start with 1 digit
    loop {
        let digits = size.to_string().len();
        let candidate = base_len + digits;
        if candidate.to_string().len() == digits {
            size = candidate;
            break;
        }
        size = candidate;
    }
    stub.replace(placeholder, &size.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params_default<'a>() -> StubParams<'a> {
        StubParams {
            runtime_hash: "rt1",
            runtime_size: 100,
            app_hash: "app1",
            app_size: 200,
            cds_hash: None,
            cds_size: 0,
            crac_hash: None,
            crac_size: 0,
            profile: &JvmProfile::Server,
            jvm_args: &[],
        }
    }

    #[test]
    fn stub_starts_with_shebang() {
        let p = StubParams {
            runtime_hash: "abc123",
            runtime_size: 1024,
            app_hash: "def456",
            app_size: 2048,
            ..params_default()
        };
        let stub = generate(&p);
        assert!(stub.starts_with("#!/bin/sh\n"));
    }

    #[test]
    fn stub_contains_runtime_hash_and_size() {
        let p = StubParams {
            runtime_hash: "deadbeef12345678",
            runtime_size: 9999,
            app_hash: "app1234",
            app_size: 555,
            ..params_default()
        };
        let stub = generate(&p);
        assert!(stub.contains("RT_HASH=\"deadbeef12345678\""));
        assert!(stub.contains("RT_SIZE=9999"));
    }

    #[test]
    fn stub_contains_app_hash_and_size() {
        let p = StubParams {
            app_hash: "apphash99",
            app_size: 4444,
            ..params_default()
        };
        let stub = generate(&p);
        assert!(stub.contains("APP_HASH=\"apphash99\""));
        assert!(stub.contains("APP_SIZE=4444"));
    }

    #[test]
    fn stub_with_cds() {
        let p = StubParams {
            cds_hash: Some("cds123"),
            cds_size: 300,
            ..params_default()
        };
        let stub = generate(&p);
        assert!(stub.contains("CDS_SIZE=300"));
        assert!(stub.contains("CDS_HASH=\"cds123\""));
    }

    #[test]
    fn stub_without_cds() {
        let stub = generate(&params_default());
        assert!(stub.contains("CDS_SIZE=0"));
        assert!(stub.contains("CDS_HASH=\"\""));
    }

    #[test]
    fn stub_with_crac() {
        let p = StubParams {
            crac_hash: Some("crac1"),
            crac_size: 500,
            ..params_default()
        };
        let stub = generate(&p);
        assert!(stub.contains("CRAC_SIZE=500"));
        assert!(stub.contains("CRAC_HASH=\"crac1\""));
    }

    #[test]
    fn stub_cli_profile_flags() {
        let profile = JvmProfile::Cli;
        let p = StubParams {
            profile: &profile,
            ..params_default()
        };
        let stub = generate(&p);
        assert!(stub.contains("-XX:+TieredCompilation"));
        assert!(stub.contains("-XX:TieredStopAtLevel=1"));
        assert!(stub.contains("-XX:+UseSerialGC"));
    }

    #[test]
    fn stub_server_profile_no_extra_flags() {
        let stub = generate(&params_default());
        assert!(!stub.contains("-XX:+TieredCompilation"));
        assert!(!stub.contains("TieredStopAtLevel"));
        assert!(!stub.contains("UseSerialGC"));
    }

    #[test]
    fn stub_with_jvm_args() {
        let args = vec!["-Xmx512m".to_string(), "-Dapp.env=prod".to_string()];
        let p = StubParams {
            jvm_args: &args,
            ..params_default()
        };
        let stub = generate(&p);
        assert!(stub.contains("-Xmx512m -Dapp.env=prod"));
    }

    #[test]
    fn stub_cli_profile_with_jvm_args() {
        let profile = JvmProfile::Cli;
        let args = vec!["-Xmx256m".to_string()];
        let p = StubParams {
            profile: &profile,
            jvm_args: &args,
            ..params_default()
        };
        let stub = generate(&p);
        assert!(stub
            .contains("-XX:+TieredCompilation -XX:TieredStopAtLevel=1 -XX:+UseSerialGC -Xmx256m"));
    }

    #[test]
    fn stub_ends_with_payload_marker() {
        let stub = generate(&params_default());
        assert!(stub.ends_with("# --- PAYLOAD BELOW ---\n"));
    }

    #[test]
    fn stub_contains_banner() {
        let stub = generate(&params_default());
        assert!(stub.contains("BANNER"));
        assert!(stub.contains("(_) |__"));
    }

    #[test]
    fn stub_contains_layered_cache_dirs() {
        let stub = generate(&params_default());
        assert!(stub.contains("rt-$RT_HASH"));
        assert!(stub.contains("app-$APP_HASH"));
    }

    #[test]
    fn finalize_stub_replaces_placeholder() {
        let stub = generate(&params_default());
        let finalized = finalize_stub(&stub);
        assert!(!finalized.contains("__STUB_SIZE__"));
        assert!(finalized.contains("STUB_SIZE="));
    }
}
