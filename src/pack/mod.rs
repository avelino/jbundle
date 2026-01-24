pub mod archive;
pub mod stub;

use std::io::Write;
use std::path::Path;

use flate2::write::GzEncoder;
use flate2::Compression;

use crate::config::JvmProfile;
use crate::error::PackError;

pub struct PackOptions<'a> {
    pub runtime_dir: &'a Path,
    pub jar_path: &'a Path,
    pub crac_path: Option<&'a Path>,
    pub output: &'a Path,
    pub jvm_args: &'a [String],
    pub profile: &'a JvmProfile,
    pub appcds: bool,
    pub java_version: u8,
}

pub fn create_binary(opts: &PackOptions) -> Result<(), PackError> {
    let temp = tempfile::tempdir()?;

    // Create runtime archive
    let runtime_archive = archive::create_runtime_archive(opts.runtime_dir, temp.path())?;
    let runtime_size = std::fs::metadata(&runtime_archive)?.len();
    let runtime_hash = archive::hash_file(&runtime_archive)?;

    // Compress app.jar with gzip
    let app_gz_path = temp.path().join("app.jar.gz");
    compress_file(opts.jar_path, &app_gz_path)?;
    let app_size = std::fs::metadata(&app_gz_path)?.len();
    let app_hash = archive::hash_file(opts.jar_path)?; // hash the original jar for cache identity

    // CRaC checkpoint (tar.gz)
    let (crac_size, crac_hash) = if let Some(cp) = opts.crac_path {
        let size = std::fs::metadata(cp)?.len();
        let hash = archive::hash_file(cp)?;
        (size, Some(hash))
    } else {
        (0, None)
    };

    // Generate stub
    let stub_script = stub::generate(&stub::StubParams {
        runtime_hash: &runtime_hash,
        runtime_size,
        app_hash: &app_hash,
        app_size,
        crac_hash: crac_hash.as_deref(),
        crac_size,
        profile: opts.profile,
        jvm_args: opts.jvm_args,
        appcds: opts.appcds,
        java_version: opts.java_version,
    });
    let stub_script = stub::finalize_stub(&stub_script);

    if let Some(parent) = opts.output.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut out_file = std::fs::File::create(opts.output)?;

    // Write stub
    out_file.write_all(stub_script.as_bytes())?;

    // Write runtime.tar.gz (streaming)
    let mut runtime_file = std::fs::File::open(&runtime_archive)?;
    std::io::copy(&mut runtime_file, &mut out_file)?;

    // Write app.jar.gz (streaming)
    let mut app_file = std::fs::File::open(&app_gz_path)?;
    std::io::copy(&mut app_file, &mut out_file)?;

    // Write CRaC checkpoint tar.gz (streaming, if present)
    if let Some(cp) = opts.crac_path {
        let mut crac_file = std::fs::File::open(cp)?;
        std::io::copy(&mut crac_file, &mut out_file)?;
    }

    drop(out_file);

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(opts.output, std::fs::Permissions::from_mode(0o755))?;
    }

    tracing::info!("binary created at {}", opts.output.display());
    Ok(())
}

fn compress_file(input: &Path, output: &Path) -> Result<(), PackError> {
    let mut src = std::fs::File::open(input)?;
    let dst = std::fs::File::create(output)?;
    let mut encoder = GzEncoder::new(dst, Compression::default());
    std::io::copy(&mut src, &mut encoder)?;
    encoder.finish()?;
    Ok(())
}
