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
    pub compact_banner: bool,
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
    let jbundle_banner = if params.compact_banner {
        r#"echo "binary created with jbundle.avelino.run" >&2"#
    } else {
        r#"cat >&2 <<'BANNER'
   _ _                    _ _
  (_) |__  _   _ _ __   __| | | ___
  | | '_ \| | | | '_ \ / _` | |/ _ \
  | | |_) | |_| | | | | (_| | |  __/
 _/ |_.__/ \__,_|_| |_|\__,_|_|\___|
|__/
BANNER"#
    };

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

{jbundle_banner}

STUB_SIZE=__STUB_SIZE__

# Every cache entry is extracted into a private temp dir next to its final
# location and then renamed into place, so a second copy started at the same
# time never sees a half-written runtime or app.jar.
mkdir -p "$CACHE"

# A cache entry is only trusted once it carries the .jbundle-complete marker,
# which is written inside the temp dir after extraction succeeds. Entries
# without it (killed mid-extraction, or left by an older stub that extracted
# in place) get rebuilt once.
#
# Move a freshly extracted temp dir into place. If another copy already
# installed a complete entry, drop ours. A markerless leftover is moved aside
# first so the rename lands on a free path.
jb_install_dir() {{
    if [ ! -f "$2/.jbundle-complete" ] && [ -e "$2" ]; then
        JB_STALE=$(mktemp -d "$2.stale.XXXXXX")
        mv "$2" "$JB_STALE/" 2>/dev/null || true
        rm -rf "$JB_STALE"
    fi
    mv "$1" "$2" 2>/dev/null || true
    rm -rf "$2/${{1##*/}}" "$1"
}}

# Extract runtime (only if not cached)
RT_DIR="$CACHE/rt-$RT_HASH"
if [ ! -f "$RT_DIR/.jbundle-complete" ]; then
    echo "Extracting runtime (first run)..." >&2
    RT_TMP=$(mktemp -d "$CACHE/.rt-$RT_HASH.XXXXXX")
    tail -c +$((STUB_SIZE + 1)) "$0" | head -c "$RT_SIZE" | tar xzf - -C "$RT_TMP"
    : > "$RT_TMP/.jbundle-complete"
    jb_install_dir "$RT_TMP" "$RT_DIR"
fi

# Extract app.jar (decompress gzip, only if not cached). The rename is atomic
# and the marker is written after it, so a jar without marker is redone.
APP_DIR="$CACHE/app-$APP_HASH"
if [ ! -f "$APP_DIR/.jbundle-complete" ]; then
    mkdir -p "$APP_DIR"
    APP_TMP=$(mktemp "$APP_DIR/.app.jar.XXXXXX")
    tail -c +$((STUB_SIZE + RT_SIZE + 1)) "$0" | head -c "$APP_SIZE" | gzip -d > "$APP_TMP"
    mv -f "$APP_TMP" "$APP_DIR/app.jar"
    : > "$APP_DIR/.jbundle-complete"
fi
{cds_flags}

# CRaC restore (Linux only)
if [ "$CRAC_SIZE" -gt 0 ] 2>/dev/null && [ "$(uname)" = "Linux" ]; then
    CRAC_DIR="$CACHE/crac-$CRAC_HASH"
    if [ ! -f "$CRAC_DIR/.jbundle-complete" ]; then
        CRAC_TMP=$(mktemp -d "$CACHE/.crac-$CRAC_HASH.XXXXXX")
        tail -c +$((STUB_SIZE + RT_SIZE + APP_SIZE + 1)) "$0" | head -c "$CRAC_SIZE" | tar xzf - -C "$CRAC_TMP"
        : > "$CRAC_TMP/.jbundle-complete"
        jb_install_dir "$CRAC_TMP" "$CRAC_DIR"
    fi
    "$RT_DIR/bin/java" -XX:CRaCRestoreFrom="$CRAC_DIR/cr" "$@" && exit 0
    echo "warn: CRaC restore failed, falling back to standard startup" >&2
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
            compact_banner: false,
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
    #[test]
    fn stub_extracts_via_temp_dir_and_rename() {
        let stub = generate(&params_default());
        assert!(stub.contains("mktemp -d \"$CACHE/.rt-$RT_HASH.XXXXXX\""));
        assert!(stub.contains("mv -f \"$APP_TMP\" \"$APP_DIR/app.jar\""));
        assert!(!stub.contains("tar xzf - -C \"$RT_DIR\""));
        assert!(stub.contains(r#"[ ! -f "$RT_DIR/.jbundle-complete" ]"#));
        assert!(stub.contains(r#": > "$RT_TMP/.jbundle-complete""#));
    }

    /// Start several copies of a packed binary at the same time on an empty
    /// cache. Before extraction went through a temp dir, a second copy could
    /// see `bin/` while `lib/` was still being written and launch a broken
    /// runtime. The fake `java` checks that the whole runtime and app.jar are
    /// in place before it reports success.
    #[cfg(unix)]
    #[test]
    fn concurrent_first_runs_never_see_partial_extraction() {
        use std::io::Write;
        use std::os::unix::fs::PermissionsExt;
        use std::process::{Command, Stdio};

        let work = tempfile::tempdir().unwrap();

        // Fake runtime: bin/java plus a lot of lib/ files so extraction takes a while.
        let rt_src = work.path().join("rt");
        std::fs::create_dir_all(rt_src.join("bin")).unwrap();
        std::fs::create_dir_all(rt_src.join("lib")).unwrap();
        let java = rt_src.join("bin/java");
        std::fs::write(
            &java,
            "#!/bin/sh\nD=$(dirname \"$0\")/..\n\
             [ -f \"$D/lib/zz-last\" ] || { echo partial-runtime; exit 3; }\n\
             for a in \"$@\"; do J=$a; done\n\
             [ \"$(cat \"$J\")\" = app-ok ] || { echo partial-jar; exit 4; }\n\
             echo ok\n",
        )
        .unwrap();
        std::fs::set_permissions(&java, std::fs::Permissions::from_mode(0o755)).unwrap();
        for i in 0..400 {
            std::fs::write(rt_src.join(format!("lib/f{i:04}")), vec![b'x'; 4096]).unwrap();
        }
        std::fs::write(rt_src.join("lib/zz-last"), b"done").unwrap();

        let rt_tgz = work.path().join("rt.tar.gz");
        {
            let f = std::fs::File::create(&rt_tgz).unwrap();
            let enc = flate2::write::GzEncoder::new(f, flate2::Compression::fast());
            let mut tar = tar::Builder::new(enc);
            tar.append_dir_all(".", &rt_src).unwrap();
            tar.into_inner().unwrap().finish().unwrap();
        }
        let mut app_gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        app_gz.write_all(b"app-ok").unwrap();
        let app_gz = app_gz.finish().unwrap();
        let rt_bytes = std::fs::read(&rt_tgz).unwrap();

        let stub = finalize_stub(&generate(&StubParams {
            runtime_size: rt_bytes.len() as u64,
            app_size: app_gz.len() as u64,
            appcds: false,
            ..params_default()
        }));
        let bin = work.path().join("app");
        let mut out = std::fs::File::create(&bin).unwrap();
        out.write_all(stub.as_bytes()).unwrap();
        out.write_all(&rt_bytes).unwrap();
        out.write_all(&app_gz).unwrap();
        drop(out);
        std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();

        for round in 0..5 {
            let home = work.path().join(format!("home{round}"));
            std::fs::create_dir_all(&home).unwrap();
            let children: Vec<_> = (0..4)
                .map(|_| {
                    Command::new(&bin)
                        .env("HOME", &home)
                        .stdout(Stdio::piped())
                        .stderr(Stdio::piped())
                        .spawn()
                        .unwrap()
                })
                .collect();
            for child in children {
                let o = child.wait_with_output().unwrap();
                assert!(
                    o.status.success(),
                    "round {round}: {}\n{}",
                    String::from_utf8_lossy(&o.stdout),
                    String::from_utf8_lossy(&o.stderr)
                );
                assert_eq!(String::from_utf8_lossy(&o.stdout).trim(), "ok");
            }
            let cache = home.join(".jbundle/cache");
            let leftovers: Vec<_> = std::fs::read_dir(&cache)
                .unwrap()
                .map(|e| e.unwrap().file_name().into_string().unwrap())
                .filter(|n| n.starts_with('.'))
                .collect();
            assert!(leftovers.is_empty(), "temp dirs left behind: {leftovers:?}");
            let nested: Vec<_> = std::fs::read_dir(cache.join("rt-rt1"))
                .unwrap()
                .map(|e| e.unwrap().file_name().into_string().unwrap())
                .filter(|n| n.starts_with('.') && n != ".jbundle-complete")
                .collect();
            assert!(
                nested.is_empty(),
                "loser temp dir nested in runtime: {nested:?}"
            );
        }

        // A cache left half-written (by a killed run or an older stub that
        // extracted in place) has bin/java but no lib/ and no marker. It must
        // be rebuilt, not trusted.
        let home = work.path().join("home-legacy");
        let rt = home.join(".jbundle/cache/rt-rt1");
        std::fs::create_dir_all(rt.join("bin")).unwrap();
        std::fs::copy(&java, rt.join("bin/java")).unwrap();
        let app = home.join(".jbundle/cache/app-app1");
        std::fs::create_dir_all(&app).unwrap();
        std::fs::write(app.join("app.jar"), b"app-").unwrap();
        let o = Command::new(&bin).env("HOME", &home).output().unwrap();
        assert!(
            o.status.success(),
            "legacy cache not rebuilt: {}\n{}",
            String::from_utf8_lossy(&o.stdout),
            String::from_utf8_lossy(&o.stderr)
        );
        assert!(rt.join("lib/zz-last").is_file());
        assert!(rt.join(".jbundle-complete").is_file());
        assert!(app.join(".jbundle-complete").is_file());
    }
}
