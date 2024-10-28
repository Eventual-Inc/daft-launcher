use std::{ffi::OsStr, io::ErrorKind, path::Path};

use anyhow::Context;
use console::style;
use dirs::home_dir;
use log::Level;
use tempdir::TempDir;
use tokio::fs::{File, OpenOptions};

use crate::{path_ref, PathRef};

pub fn is_debug() -> bool {
    log::log_enabled!(Level::Debug)
}

pub fn expand(path: PathRef) -> anyhow::Result<PathRef> {
    let path = if path.starts_with("~") {
        let home = home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
        let suffix = path.strip_prefix("~").unwrap();
        path_ref(home.join(suffix))
    } else {
        path
    };
    Ok(path)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    DoesNotExist,
    File,
    Directory,
}

pub async fn file_status(path: &Path) -> anyhow::Result<Status> {
    match tokio::fs::metadata(path).await {
        Ok(metadata) => {
            if metadata.is_file() {
                Ok(Status::File)
            } else if metadata.is_dir() {
                Ok(Status::Directory)
            } else {
                anyhow::bail!("The path {:?} is neither a file nor a directory", path)
            }
        }
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(Status::DoesNotExist),
        Err(error) => Err(anyhow::Error::new(error)),
    }
}

pub async fn assert_file_status(path: &Path, expected_status: Status) -> anyhow::Result<()> {
    let actual_status = file_status(path).await?;
    match (actual_status, expected_status) {
        (_, Status::DoesNotExist) => anyhow::bail!(
            "The file/dir {} already exists",
            style(path.display()).red()
        ),
        (Status::DoesNotExist, _) => anyhow::bail!(
            "The file/dir {} does not exist",
            style(path.display()).red()
        ),

        (Status::File, Status::Directory) => anyhow::bail!(
            "Expected a directory at the path {}, but found a file",
            path.display()
        ),
        (Status::Directory, Status::File) => anyhow::bail!(
            "Expected a file at the path {}, but found a directory",
            path.display()
        ),

        _ => Ok(()),
    }
}

pub async fn create_new_file(path: &Path) -> anyhow::Result<File> {
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .await
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

pub async fn create_temporary_ray_file() -> anyhow::Result<(TempDir, PathRef, File)> {
    let temp_dir =
        TempDir::new("ray_config").expect("Creation of temporary directory should always succeed");
    let path = path_ref(temp_dir.path().join("ray.yaml"));
    log::debug!(
        "Created new temporary dir {temp_dir:?} and new temporary ray config file at path {path:?}"
    );
    let file = create_new_file(&path)
        .await
        .expect("Creating new file in temporary directory should always succeed");
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
