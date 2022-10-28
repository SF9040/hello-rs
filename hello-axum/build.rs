#[cfg(feature = "proxy")]
use anyhow::Context;
use anyhow::Result;
#[cfg(feature = "proxy")]
use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
    vec,
};
#[cfg(feature = "proxy")]
use walkdir::WalkDir;

#[cfg(feature = "proxy")]
const PROTOS: &str = "proto";

fn main() -> Result<()> {
    #[cfg(feature = "proxy")]
    tonic_build()?;
    Ok(())
}

#[cfg(feature = "proxy")]
fn tonic_build() -> Result<()> {
    let protos = list_protos(Path::new(PROTOS))?;
    tonic_build::configure()
        .build_server(false)
        .compile(&protos, &[PROTOS])
        .context("Cannot compile protos")
}

#[cfg(feature = "proxy")]
fn list_protos(dir: &Path) -> Result<Vec<PathBuf>> {
    WalkDir::new(dir)
        .into_iter()
        .try_fold(vec![], |mut protos, entry| {
            let entry = entry.context("Cannot read proto file")?;
            let path = entry.path();
            if path.extension().and_then(OsStr::to_str) == Some("proto") {
                protos.push(path.to_path_buf());
            }
            Ok(protos)
        })
}
