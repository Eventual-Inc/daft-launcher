use tokio::fs;

use super::*;

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
/// This does *not* check the contents of the newly created configuration file.
/// The reason is because we perform some minor templatization of the
/// `template.toml` file before writing it. Thus, the outputted configuration
/// file does not *exactly* match the original `template.toml` file.
#[tokio::test]
async fn test_init() {
    let (_temp_dir, path) = get_path().await;

    run(DaftLauncher {
        sub_command: SubCommand::Init(Init {
            init_provider: InitProvider::Aws,
            path: path.clone(),
        }),
        verbosity: 0,
    })
    .await
    .unwrap();

    assert!(path.exists());
    assert!(path.is_file());
}

/// Tests to make sure that `daft check` properly asserts the schema of the
/// newly created daft-launcher configuration file.
#[tokio::test]
async fn test_check() {
    let (_temp_dir, path) = get_path().await;

    run(DaftLauncher {
        sub_command: SubCommand::Init(Init {
            init_provider: InitProvider::K8s,
            path: path.clone(),
        }),
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

/// This tests the core conversion functionality, from a `DaftConfig` to a
/// `RayConfig`.
///
/// # Note
/// Fields which expect a filesystem path (i.e., "ssh_private_key" and
/// "job.working_dir") are not checked for existence. Therefore, you can really
/// put any value in there and this test will pass.
///
/// This is because the point of this test is not to check for existence, but
/// rather to test the mapping from `DaftConfig` to `RayConfig`.
#[rstest::rstest]
#[case(simple_config())]
fn test_conversion(
    #[case] (daft_config, teardown_behaviour, expected): (
        DaftConfig,
        Option<TeardownBehaviour>,
        RayConfig,
    ),
) {
    let actual = convert_to_ray_config(&daft_config, teardown_behaviour).unwrap();
    assert_eq!(actual, expected);
}

#[rstest::rstest]
#[case("3.9".parse().unwrap(), "2.34".parse().unwrap(), vec![], vec![
    "curl -LsSf https://astral.sh/uv/install.sh | sh".into(),
    "uv python install 3.9".into(),
    "uv python pin 3.9".into(),
    "uv venv".into(),
    "echo 'source $HOME/.venv/bin/activate' >> ~/.bashrc".into(),
    "source ~/.bashrc".into(),
    r#"uv pip install boto3 pip py-spy deltalake getdaft "ray[default]==2.34""#.into(),
])]
#[case("3.9".parse().unwrap(), "2.34".parse().unwrap(), vec!["requests==0.0.0".into()], vec![
    "curl -LsSf https://astral.sh/uv/install.sh | sh".into(),
    "uv python install 3.9".into(),
    "uv python pin 3.9".into(),
    "uv venv".into(),
    "echo 'source $HOME/.venv/bin/activate' >> ~/.bashrc".into(),
    "source ~/.bashrc".into(),
    r#"uv pip install boto3 pip py-spy deltalake getdaft "ray[default]==2.34""#.into(),
    r#"uv pip install "requests==0.0.0""#.into(),
])]
fn test_generate_setup_commands(
    #[case] python_version: Versioning,
    #[case] ray_version: Versioning,
    #[case] dependencies: Vec<StrRef>,
    #[case] expected: Vec<StrRef>,
) {
    let actual = generate_setup_commands(python_version, ray_version, dependencies.as_slice());
    assert_eq!(actual, expected);
}

#[rstest::fixture]
pub fn simple_config() -> (DaftConfig, Option<TeardownBehaviour>, RayConfig) {
    let test_name: StrRef = "test".into();
    let ssh_private_key: PathRef = Arc::from(PathBuf::from("testkey.pem"));
    let number_of_workers = 4;
    let daft_config = DaftConfig {
        setup: DaftSetup {
            name: test_name.clone(),
            requires: "=1.2.3".parse().unwrap(),
            python_version: "3.12".parse().unwrap(),
            ray_version: "2.34".parse().unwrap(),
            provider_config: ProviderConfig::Aws(AwsConfig {
                region: test_name.clone(),
                number_of_workers,
                ssh_user: test_name.clone(),
                ssh_private_key: ssh_private_key.clone(),
                instance_type: test_name.clone(),
                image_id: test_name.clone(),
                iam_instance_profile_name: Some(test_name.clone()),
                dependencies: vec![],
                run: vec![],
            }),
        },
        jobs: HashMap::default(),
    };
    let node_config = RayNodeConfig {
        key_name: "testkey".into(),
        instance_type: test_name.clone(),
        image_id: test_name.clone(),
        iam_instance_profile: Some(IamInstanceProfile {
            name: test_name.clone(),
        }),
    };

    let ray_config = RayConfig {
        cluster_name: test_name.clone(),
        max_workers: number_of_workers,
        provider: RayProvider {
            r#type: "aws".into(),
            region: test_name.clone(),
            cache_stopped_nodes: None,
        },
        auth: RayAuth {
            ssh_user: test_name.clone(),
            ssh_private_key,
        },
        available_node_types: vec![
            (
                "ray.head.default".into(),
                RayNodeType {
                    max_workers: 0,
                    node_config: node_config.clone(),
                    resources: Some(RayResources { cpu: 0 }),
                },
            ),
            (
                "ray.worker.default".into(),
                RayNodeType {
                    max_workers: number_of_workers,
                    node_config,
                    resources: None,
                },
            ),
        ]
        .into_iter()
        .collect(),
        setup_commands: vec![
            "curl -LsSf https://astral.sh/uv/install.sh | sh".into(),
            "uv python install 3.12".into(),
            "uv python pin 3.12".into(),
            "uv venv".into(),
            "echo 'source $HOME/.venv/bin/activate' >> ~/.bashrc".into(),
            "source ~/.bashrc".into(),
            r#"uv pip install boto3 pip py-spy deltalake getdaft "ray[default]==2.34""#.into(),
        ],
    };

    (daft_config, None, ray_config)
}
