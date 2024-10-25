use std::{
    ffi::OsStr,
    fs::{File, OpenOptions},
    io::ErrorKind,
    path::Path,
};

use anyhow::Context;
use dirs::home_dir;
use log::Level;
use tempdir::TempDir;

use crate::{path_ref, PathRef};

pub fn is_debug() -> bool {
    log::log_enabled!(Level::Debug)
}

pub fn expand(path: PathRef) -> anyhow::Result<PathRef> {
    let path = if path.starts_with("~") {
        let home = home_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
        let suffix = path.strip_prefix("~").unwrap();
        path_ref(home.join(suffix))
    } else {
        path
    };
    Ok(path)
}

pub fn assert_file_existence_status(
    path: &Path,
    should_exist: bool,
) -> anyhow::Result<()> {
    let exists = path.exists();
    match (exists, should_exist) {
        (true, true) | (false, false) => Ok(()),
        (true, false) => anyhow::bail!("The file {:?} already exists", path),
        (false, true) => anyhow::bail!("The file {:?} does not exist", path),
    }
}

pub fn create_new_file(path: &Path) -> anyhow::Result<File> {
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| match error.kind() {
            ErrorKind::AlreadyExists => {
                anyhow::anyhow!("The file {:?} already exists", path)
            }
            _ => error.into(),
        })?;
    log::debug!("Created new file: {path:?}");
    Ok(file)
}

pub fn path_to_str<'a, I>(path: I) -> anyhow::Result<&'a str>
where
    I: 'a,
    &'a OsStr: From<I>,
{
    let path: &OsStr = path.into();
    path.to_str()
        .with_context(|| anyhow::anyhow!("Invalid characters in path"))
}

pub fn create_ray_temporary_file() -> anyhow::Result<(TempDir, PathRef, File)> {
    let temp_dir = TempDir::new("ray_config")
        .expect("Creation of temporary directory should always succeed");
    let path = path_ref(temp_dir.path().join("ray.yaml"));
    log::debug!("Created new temporary dir {temp_dir:?} and new temporary ray config file at path {path:?}");
    let file = create_new_file(&path).expect(
        "Creating new file in temporary directory should always succeed",
    );
    Ok((temp_dir, path, file))
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::{path_ref, StrRef};

    #[rstest]
    #[case("target", "target".into())]
    #[case(".ssh", ".ssh".into())]
    #[case("~/.ssh", format!("{}/.ssh", env!("HOME")).into())]
    fn test_expansion(#[case] path: &str, #[case] expected: StrRef) {
        let path = path_ref(path);
        let expected = path_ref(&*expected);
        let actual = expand(path).unwrap();
        assert_eq!(&*actual, &*expected);
    }
}
