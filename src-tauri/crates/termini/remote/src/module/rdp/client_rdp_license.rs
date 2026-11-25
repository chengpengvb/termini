use sha1::Sha1;
use std::path::PathBuf;
use sha1::Digest as Sha1Digest;
use crate::module::rdp::protocol::core::error::{RdpProtocolError, RdpResult};

const DATA_DIR_ENV: &str = "TERMINI_DATA_DIR";
const LICENSE_DIR: &str = "rdp";
const LICENSE_STORE: &str = "licenses";
const LICENSE_EXTENSION: &str = "cal";
const STAGING_EXTENSION: &str = "cal.new";

pub(crate) fn load(machine_name: &str) -> RdpResult<Option<Vec<u8>>> {
    let path = license_path(machine_name)?;
    if !path.exists() {
        return Ok(None);
    }
    let license = std::fs::read(&path).map_err(|error| RdpProtocolError::protocol(format!("failed to read license file {:?}: {}", path, error)))?;
    Ok(Some(license))
}

pub(crate) fn store(machine_name: &str, license: &[u8]) -> RdpResult<()> {
    let path = license_path(machine_name)?;
    let directory = path.parent().ok_or_else(|| RdpProtocolError::protocol(format!("license file path has no parent directory: {:?}", path)))?;
    std::fs::create_dir_all(directory).map_err(|error| RdpProtocolError::protocol(format!("failed to create license directory {:?}: {}", directory, error)))?;
    let staging = path.with_extension(STAGING_EXTENSION);
    std::fs::write(&staging, license).map_err(|error| RdpProtocolError::protocol(format!("failed to write license file {:?}: {}", staging, error)))?;
    std::fs::rename(&staging, &path).map_err(|error| RdpProtocolError::protocol(format!("failed to commit license file {:?}: {}", path, error)))
}

fn license_path(machine_name: &str) -> RdpResult<PathBuf> {
    let data_dir = std::env::var(DATA_DIR_ENV).map_err(|error| RdpProtocolError::protocol(format!("failed to read environment variable {}: {}", DATA_DIR_ENV, error)))?;
    let digest = Sha1::digest(machine_name.as_bytes());
    let name = digest.iter().map(|byte| format!("{:02x}", byte)).collect::<String>();
    Ok(PathBuf::from(data_dir).join(LICENSE_DIR).join(LICENSE_STORE).join(name).with_extension(LICENSE_EXTENSION))
}
