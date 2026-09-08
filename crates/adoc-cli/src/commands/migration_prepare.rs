use adoc_core::{MIGRATION_REQUEST_MAX_BYTES, MigrationError};
use std::{
    fs::File,
    io::{Read, Write},
    path::PathBuf,
};

pub(crate) fn migration_prepare(
    request: PathBuf,
    repository: PathBuf,
    runtime_binary_digest: String,
) -> i32 {
    let result = (|| {
        let file = File::open(request).map_err(|_| MigrationError::InvalidRequest)?;
        if !file
            .metadata()
            .map_err(|_| MigrationError::InvalidRequest)?
            .is_file()
        {
            return Err(MigrationError::InvalidRequest);
        }
        let mut bytes = Vec::new();
        file.take(MIGRATION_REQUEST_MAX_BYTES as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| MigrationError::InvalidRequest)?;
        let receipt = adoc_local::prepare_migration(
            &repository,
            &bytes,
            env!("CARGO_PKG_VERSION").into(),
            runtime_binary_digest,
        )?;
        let output = receipt
            .to_canonical_json()
            .map_err(|_| MigrationError::ValidationUnavailable)?;
        std::io::stdout()
            .write_all(output.as_bytes())
            .map_err(|_| MigrationError::ValidationUnavailable)?;
        Ok(match receipt.result() {
            adoc_core::ValidationResult::Pass => 0,
            adoc_core::ValidationResult::Fail => 1,
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
