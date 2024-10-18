use std::{
    ffi::OsStr,
    fs::{File, OpenOptions},
    io::ErrorKind,
    path::Path,
};

use anyhow::Context;
use dirs::home_dir;
use tempdir::TempDir;

use crate::{path_ref, PathRef};

pub fn expand(path: PathRef) -> anyhow::Result<PathRef> {
    if path.starts_with("~") {
        let home = home_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
        let suffix = path.strip_prefix("~").unwrap();
        Ok(path_ref(home.join(suffix)))
    } else {
        Ok(path)
    }
}

pub async fn is_authenticated_with_aws() -> bool {
    // let sdk_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
    //     .region(aws_config::meta::region::RegionProviderChain::default_provider())
    //     .load()
    //     .await;
    // let client = aws_sdk_sts::Client::new(&sdk_config);
    // client.get_caller_identity().send().await.is_ok()
    true
}

pub async fn assert_is_authenticated_with_aws() -> anyhow::Result<()> {
    if is_authenticated_with_aws().await {
        Ok(())
    } else {
        anyhow::bail!("You are not signed in to AWS; please sign in first")
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
    log::info!("Created new file: {path:?}");
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
    log::info!("Created new temporary dir {temp_dir:?} and new temporary ray config file at path {path:?}");
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
        assert_eq!(actual, expected);
    }
}
