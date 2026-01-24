use crate::config::JvmProfile;

pub struct StubParams<'a> {
    pub runtime_hash: &'a str,
    pub runtime_size: u64,
    pub app_hash: &'a str,
    pub app_size: u64,
    pub crac_hash: Option<&'a str>,
    pub crac_size: u64,
    pub profile: &'a JvmProfile,
    pub jvm_args: &'a [String],
    pub appcds: bool,
    pub java_version: u8,
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

    let crac_hash_val = params.crac_hash.unwrap_or("");

    let runtime_hash = params.runtime_hash;
    let runtime_size = params.runtime_size;
    let app_hash = params.app_hash;
    let app_size = params.app_size;
    let crac_size = params.crac_size;

    // AppCDS via AutoCreateSharedArchive (JDK 19+)
    let cds_flags = if params.appcds && params.java_version >= 19 {
        r#"
# AppCDS: auto-create shared archive on first run (JDK 19+)
CDS_FILE="$APP_DIR/app.jsa"
CDS_FLAG="-XX:+AutoCreateSharedArchive -XX:SharedArchiveFile=$CDS_FILE""#
    } else {
        "\nCDS_FLAG=\"\""
    };

    format!(
        r#"#!/bin/sh
set -e
CACHE="${{HOME}}/.jbundle/cache"
RT_HASH="{runtime_hash}"    RT_SIZE={runtime_size}
APP_HASH="{app_hash}"   APP_SIZE={app_size}
CRAC_SIZE={crac_size}       CRAC_HASH="{crac_hash_val}"

cat >&2 <<'BANNER'
   _ _                    _ _
  (_) |__  _   _ _ __   __| | | ___
  | | '_ \| | | | '_ \ / _` | |/ _ \
  | | |_) | |_| | | | | (_| | |  __/
 _/ |_.__/ \__,_|_| |_|\__,_|_|\___|
|__/
BANNER

STUB_SIZE=__STUB_SIZE__

# Extract runtime (only if not cached)
RT_DIR="$CACHE/rt-$RT_HASH"
if [ ! -d "$RT_DIR/bin" ]; then
    mkdir -p "$RT_DIR"
    echo "Extracting runtime (first run)..." >&2
    tail -c +$((STUB_SIZE + 1)) "$0" | head -c "$RT_SIZE" | tar xzf - -C "$RT_DIR"
fi

# Extract app.jar (decompress gzip, only if not cached)
APP_DIR="$CACHE/app-$APP_HASH"
if [ ! -f "$APP_DIR/app.jar" ]; then
    mkdir -p "$APP_DIR"
    tail -c +$((STUB_SIZE + RT_SIZE + 1)) "$0" | head -c "$APP_SIZE" | gzip -d > "$APP_DIR/app.jar"
fi
{cds_flags}

# CRaC restore (Linux only)
if [ "$CRAC_SIZE" -gt 0 ] 2>/dev/null && [ "$(uname)" = "Linux" ]; then
    CRAC_DIR="$CACHE/crac-$CRAC_HASH"
    if [ ! -d "$CRAC_DIR/cr" ]; then
        mkdir -p "$CRAC_DIR"
        tail -c +$((STUB_SIZE + RT_SIZE + APP_SIZE + 1)) "$0" | head -c "$CRAC_SIZE" | tar xzf - -C "$CRAC_DIR"
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
            crac_hash: None,
            crac_size: 0,
            profile: &JvmProfile::Server,
            jvm_args: &[],
            appcds: true,
            java_version: 21,
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
    fn stub_with_appcds_jdk21() {
        let p = StubParams {
            appcds: true,
            java_version: 21,
            ..params_default()
        };
        let stub = generate(&p);
        assert!(stub.contains("AutoCreateSharedArchive"));
        assert!(stub.contains("SharedArchiveFile"));
    }

    #[test]
    fn stub_without_appcds() {
        let p = StubParams {
            appcds: false,
            ..params_default()
        };
        let stub = generate(&p);
        assert!(!stub.contains("AutoCreateSharedArchive"));
    }

    #[test]
    fn stub_appcds_disabled_for_old_jdk() {
        let p = StubParams {
            appcds: true,
            java_version: 17,
            ..params_default()
        };
        let stub = generate(&p);
        assert!(!stub.contains("AutoCreateSharedArchive"));
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
    fn stub_decompresses_app_jar() {
        let stub = generate(&params_default());
        assert!(stub.contains("gzip -d"));
    }

    #[test]
    fn finalize_stub_replaces_placeholder() {
        let stub = generate(&params_default());
        let finalized = finalize_stub(&stub);
        assert!(!finalized.contains("__STUB_SIZE__"));
        assert!(finalized.contains("STUB_SIZE="));
    }
}
