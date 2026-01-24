pub mod archive;
pub mod stub;

use std::path::Path;

use crate::config::JvmProfile;
use crate::error::PackError;

pub fn create_binary(
    runtime_dir: &Path,
    jar_path: &Path,
    appcds_path: Option<&Path>,
    crac_path: Option<&Path>,
    output: &Path,
    jvm_args: &[String],
    profile: &JvmProfile,
) -> Result<(), PackError> {
    let temp = tempfile::tempdir()?;

    // Create runtime archive
    let runtime_archive = archive::create_runtime_archive(runtime_dir, temp.path())?;
    let runtime_size = std::fs::metadata(&runtime_archive)?.len();
    let runtime_hash = archive::hash_file(&runtime_archive)?;

    // Read app.jar (stored raw, not archived)
    let app_size = std::fs::metadata(jar_path)?.len();
    let app_hash = archive::hash_file(jar_path)?;

    // AppCDS (.jsa file, stored raw)
    let (cds_size, cds_hash) = if let Some(cds_path) = appcds_path {
        let size = std::fs::metadata(cds_path)?.len();
        let hash = archive::hash_file(cds_path)?;
        (size, Some(hash))
    } else {
        (0, None)
    };

    // CRaC checkpoint (tar.gz)
    let (crac_size, crac_hash) = if let Some(cp) = crac_path {
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
        cds_hash: cds_hash.as_deref(),
        cds_size,
        crac_hash: crac_hash.as_deref(),
        crac_size,
        profile,
        jvm_args,
    });
    let stub_script = stub::finalize_stub(&stub_script);

    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut out_file = std::fs::File::create(output)?;
    use std::io::Write;

    // Write stub
    out_file.write_all(stub_script.as_bytes())?;

    // Write runtime.tar.gz
    let runtime_data = std::fs::read(&runtime_archive)?;
    out_file.write_all(&runtime_data)?;

    // Write app.jar (raw)
    let app_data = std::fs::read(jar_path)?;
    out_file.write_all(&app_data)?;

    // Write AppCDS .jsa (raw, if present)
    if let Some(cds_path) = appcds_path {
        let cds_data = std::fs::read(cds_path)?;
        out_file.write_all(&cds_data)?;
    }

    // Write CRaC checkpoint tar.gz (if present)
    if let Some(cp) = crac_path {
        let crac_data = std::fs::read(cp)?;
        out_file.write_all(&crac_data)?;
    }

    drop(out_file);

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(output, std::fs::Permissions::from_mode(0o755))?;
    }

    tracing::info!("binary created at {}", output.display());
    Ok(())
}
