use adoc_core::{MIGRATION_IMPORT_JOB_MAX_BYTES, MIGRATION_REQUEST_MAX_BYTES, MigrationError};
use std::{
    fs::File,
    io::{Read, Write},
    path::{Path, PathBuf},
};

pub(crate) fn migration_import(
    request: PathBuf,
    job: PathBuf,
    repository: PathBuf,
    runtime_binary_digest: String,
) -> i32 {
    let result: Result<i32, MigrationError> = (|| {
        let request = read_bounded(
            &request,
            MIGRATION_REQUEST_MAX_BYTES,
            MigrationError::InvalidRequest,
        )?;
        let job = read_bounded(
            &job,
            MIGRATION_IMPORT_JOB_MAX_BYTES,
            MigrationError::InvalidJob,
        )?;
        let bundle = adoc_core::import_migration_from_git(
            &repository,
            &request,
            &job,
            env!("CARGO_PKG_VERSION").into(),
            runtime_binary_digest,
        )?;
        let bytes = bundle.to_canonical_json()?;
        std::io::stdout()
            .write_all(bytes.as_bytes())
            .map_err(|_| MigrationError::ValidationUnavailable)?;
        Ok(0)
    })();
    match result {
        Ok(code) => code,
        Err(error) => {
            eprintln!("error[{}] {error}", error.diagnostic_code());
            2
        }
    }
}
pub(super) fn read_bounded(
    path: &Path,
    limit: usize,
    refusal: MigrationError,
) -> Result<Vec<u8>, MigrationError> {
    let read = || -> std::io::Result<Vec<u8>> {
        let file = File::open(path)?;
        if !file.metadata()?.is_file() {
            return Err(std::io::Error::other("regular file required"));
        }
        let mut bytes = Vec::new();
        file.take(limit as u64 + 1).read_to_end(&mut bytes)?;
        if bytes.len() > limit {
            return Err(std::io::Error::other("input limit exceeded"));
        }
        Ok(bytes)
    };
    read().map_err(|_| refusal)
}
