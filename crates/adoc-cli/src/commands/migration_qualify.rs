use super::migration_import::read_bounded;
use adoc_core::{
    MIGRATION_IMPORT_JOB_MAX_BYTES, MIGRATION_REQUEST_MAX_BYTES, MigrationError, ValidationResult,
};
use std::{io::Write, path::PathBuf};

pub(crate) fn migration_qualify(
    request: PathBuf,
    job: PathBuf,
    policy: String,
    repository: PathBuf,
    pin: String,
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
        let result = adoc_core::qualify_migration_from_git(
            &repository,
            &request,
            &job,
            &policy,
            env!("CARGO_PKG_VERSION").into(),
            pin,
        )?;
        let bytes = result.to_canonical_json()?;
        std::io::stdout()
            .write_all(bytes.as_bytes())
            .map_err(|_| MigrationError::ValidationUnavailable)?;
        Ok(if result.result() == ValidationResult::Pass {
            0
        } else {
            1
        })
    })();
    match result {
        Ok(code) => code,
        Err(error) => {
            eprintln!("error[{}] {error}", error.diagnostic_code());
            2
        }
    }
}
