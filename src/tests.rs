use super::*;
use tokio::fs;

fn not_found_okay(result: std::io::Result<()>) -> std::io::Result<()> {
    match result {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err),
    }
}

async fn get_path() -> (TempDir, PathBuf) {
    let (temp_dir, path) = create_temp_file(".test.toml").unwrap();
    not_found_okay(fs::remove_file(path.as_ref()).await).unwrap();
    not_found_okay(fs::remove_dir_all(path.as_ref()).await).unwrap();
    (temp_dir, PathBuf::from(path.as_ref()))
}

/// This tests the creation of a daft-launcher configuration file.
///
/// # Note
/// This does *not* check the contents of the newly created configuration file. The reason is
/// because we perform some minor templatization of the `template.toml` file before writing it.
/// Thus, the outputted configuration file does not *exactly* match the original `template.toml` file.
#[tokio::test]
async fn test_init() {
    let (_temp_dir, path) = get_path().await;

    run(DaftLauncher {
        sub_command: SubCommand::Init(Init { path: path.clone() }),
        verbosity: 0,
    })
    .await
    .unwrap();

    assert!(path.exists());
    assert!(path.is_file());
}

/// Tests to make sure that `daft check` properly asserts the schema of the newly created
/// daft-launcher configuration file.
#[tokio::test]
async fn test_check() {
    let (_temp_dir, path) = get_path().await;

    run(DaftLauncher {
        sub_command: SubCommand::Init(Init { path: path.clone() }),
        verbosity: 0,
    })
    .await
    .unwrap();
    run(DaftLauncher {
        sub_command: SubCommand::Check(ConfigPath { config: path }),
        verbosity: 0,
    })
    .await
    .unwrap();
}

#[rstest::fixture]
fn simple_config() -> (DaftConfig, Option<TeardownBehaviour>, RayConfig) {
    // let daft_config = DaftConfig {
    //     setup: DaftSetup,
    //     run: DaftRun,
    //     jobs: HashMap::default(),
    // };
    // let ray_config = Ra {};

    todo!()
}

#[rstest::rstest]
#[case(simple_config())]
fn test_conversion(
    #[case] (daft_config, teardown_behaviour, expected): (
        DaftConfig,
        Option<TeardownBehaviour>,
        RayConfig,
    ),
) {
    let actual = convert(&daft_config, teardown_behaviour).unwrap();
    assert_eq!(actual, expected);
}
