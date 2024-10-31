use std::{path::Path, time::Duration};

use aws_sdk_ec2::types::InstanceStateName;
use comfy_table::{
    modifiers::{UTF8_ROUND_CORNERS, UTF8_SOLID_INNER_BORDERS},
    presets::UTF8_FULL,
    Attribute, Cell, CellAlignment, Color, ContentArrangement, Table,
};
use console::style;
use dialoguer::{theme::ColorfulTheme, Input, Select};
use tokio::{fs::OpenOptions, io::AsyncWriteExt, process::Child, time};

use crate::{
    aws::{list_instances, AwsInstance},
    cli::{Init, List, Submit},
    config::{
        defaults::{normal_image_id, normal_instance_type},
        processed::{self, Job, ProcessedConfig},
        raw::{
            default_name, AwsCluster, AwsCustomType, AwsTemplateType, Cluster, Package, Provider,
            RawConfig,
        },
        ray::RayConfig,
        Selectable,
    },
    ray::{self, run_ray, RayCommand, RayJob},
    utils::{create_new_file, create_temporary_file, start_ssh_process},
    ArcStrRef, PathRef, StrRef,
};

const SQL_PY_SCRIPT: &str = include_str!(path_from_root!("assets" / "sql.py"));

pub async fn handle_init(init: Init) -> anyhow::Result<()> {
    let raw_config = if init.default {
        RawConfig::default()
    } else {
        let name = with_input("Cluster name", &prefix(NOTEPAD_EMOJI), default_name())?;
        let provider = match with_selection::<Provider>("Cloud provider", &prefix(CLOUD_EMOJI))? {
            Provider::Aws(aws_cluster) => {
                let template =
                    with_selection::<AwsTemplateType>("Template", &prefix(HAMMER_EMOJI))?;
                let custom = if template.is_none() {
                    let instance_type = with_input(
                        "Instance type",
                        &prefix(COMPUTER_EMOJI),
                        &*normal_instance_type(),
                    )?;
                    let image_id =
                        with_input("Image ID", &prefix(COMPUTER_EMOJI), &*normal_image_id())?;
                    Some(AwsCustomType {
                        instance_type,
                        image_id,
                    })
                } else {
                    None
                };
                Provider::Aws(AwsCluster {
                    template,
                    custom,
                    ..aws_cluster
                })
            }
        };
        RawConfig {
            package: Package {
                name,
                ..Default::default()
            },
            cluster: Cluster {
                provider,
                ..Default::default()
            },
            ..Default::default()
        }
    };
    let mut file = create_new_file(&init.name).await?;
    let config = toml::to_string_pretty(&raw_config).expect("Serialization should always succeed");
    let config = format!(
        r#"# For a full schema of this configuration file, please visit:
# https://eventual-inc.github.io/daft-launcher
#
# If you notice any bugs, please reach out to me (Raunak Bhagat) via our open Slack workspace, "Distributed Data Community":
# https://join.slack.com/t/dist-data/shared_invite/zt-2ric3mssh-zX08IXaKNeyx8YtqXey41A

{}"#,
        config
    );
    file.write_all(config.as_bytes()).await?;
    println!(
        "Created a new Daft Launcher config file at the path: {}",
        style(format!("`{}`", init.name.display())).cyan(),
    );
    Ok(())
}

pub async fn handle_up(
    processed_config: ProcessedConfig,
    ray_config: RayConfig,
) -> anyhow::Result<()> {
    match processed_config.cluster.provider {
        processed::Provider::Aws(ref aws_cluster) => {
            if aws_cluster.iam_instance_profile_arn.is_none() {
                log::warn!("You specified no IAM instance profile ARN; this may cause limit your cluster's abilities to interface with auxiliary AWS services");
            }
        }
    };

    let (stylized_name, stylized_cloud) = to_stylized_name_and_cloud(&processed_config);
    println!(
        r#"Spinning up a cluster named {} in your {} cloud"#,
        stylized_name, stylized_cloud
    );
    let _ = run_ray(RayCommand::Up, &ray_config).await?;
    println!(r#"Successfully spun up a cluster named {}"#, stylized_name,);
    Ok(())
}

pub async fn handle_down(
    processed_config: ProcessedConfig,
    ray_config: RayConfig,
) -> anyhow::Result<()> {
    let (stylized_name, stylized_cloud) = to_stylized_name_and_cloud(&processed_config);
    println!(
        r#"Spinning down the {} cluster in your {} cloud"#,
        stylized_name, stylized_cloud
    );
    let _ = run_ray(RayCommand::Down, &ray_config).await?;
    println!(r#"Successfully spun down the {} cluster"#, stylized_name);
    Ok(())
}

pub async fn handle_list(list: List) -> anyhow::Result<()> {
    let instances = list_instances("us-west-2").await?;
    let mut table = Table::default();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .apply_modifier(UTF8_SOLID_INNER_BORDERS)
        .set_content_arrangement(ContentArrangement::DynamicFullWidth)
        .set_header(["Name", "Instance ID", "Status", "IPv4"].map(|header| {
            Cell::new(header)
                .set_alignment(CellAlignment::Center)
                .add_attribute(Attribute::Bold)
        }));

    for instance in &instances {
        let running_condition = {
            let is_running = instance
                .state
                .as_ref()
                .map_or(false, |state| *state == InstanceStateName::Running);
            !(list.running && !is_running)
        };
        let regex_condition = list
            .name
            .as_ref()
            .map_or(true, |regex| regex.is_match(&instance.regular_name));
        let head_condition = {
            let is_head_node = instance.is_head();
            !(list.head && !is_head_node)
        };
        if running_condition && regex_condition && head_condition {
            add_instance_to_table(instance, &mut table);
        }
    }

    println!("{}", table);

    Ok(())
}

pub async fn handle_submit(
    submit: Submit,
    processed_config: ProcessedConfig,
    ray_config: RayConfig,
) -> anyhow::Result<()> {
    let (stylized_name, stylized_cloud) = to_stylized_name_and_cloud(&processed_config);
    println!(
        r#"Submitting the "{}" job to the {} cluster in your {} cloud"#,
        style(&submit.name).cyan(),
        stylized_name,
        stylized_cloud,
    );
    match processed_config.cluster.provider {
        processed::Provider::Aws(ref aws_cluster) => {
            let child = aws_ssh_helper(
                &processed_config.package.name,
                &aws_cluster.ssh_user,
                &aws_cluster.ssh_private_key,
            )
            .await?;
            let job = processed_config.jobs.get(&*submit.name).ok_or_else(|| anyhow::anyhow!(
                r#"A job with the name "{}" was not found in the config file located at {}"#,
                style(&submit.name).cyan(),
                style(&submit.config.config.display()).cyan(),
            ))?.clone();
            let _ = run_ray(RayCommand::Job(RayJob::Submit(job)), &ray_config).await?;

            drop(child);
        }
    };
    Ok(())
}

pub async fn handle_connect(
    processed_config: ProcessedConfig,
    no_dashboard: bool,
) -> anyhow::Result<()> {
    match processed_config.cluster.provider {
        processed::Provider::Aws(aws_cluster) => {
            let mut child = aws_ssh_helper(
                &processed_config.package.name,
                &aws_cluster.ssh_user,
                &aws_cluster.ssh_private_key,
            )
            .await?;
            if no_dashboard {
                child.wait().await?;
            } else {
                time::sleep(Duration::from_millis(500)).await;
                let (open_fut, child_fut) = tokio::join!(
                    async {
                        open::that(ray::LOCAL_ON_CONNECT_ADDR)?;
                        Ok::<_, anyhow::Error>(())
                    },
                    child.wait(),
                );
                open_fut?;
                child_fut?;
            }
        }
    }
    Ok(())
}

pub async fn handle_sql(
    processed_config: ProcessedConfig,
    ray_config: RayConfig,
    args: Vec<ArcStrRef>,
) -> anyhow::Result<()> {
    let sql_query = args.join(" ");
    let command = format!("python sql.py '{}'", sql_query).into();
    match processed_config.cluster.provider {
        processed::Provider::Aws(aws_cluster) => {
            let (temp_dir, path, mut file) =
                create_temporary_file("daft-launcher", "sql.py").await?;
            file.write_all(SQL_PY_SCRIPT.as_bytes()).await?;
            let working_dir: PathRef = path
                .parent()
                .expect("Path inside of temporary directory should always have a parent")
                .into();
            let child = aws_ssh_helper(
                &processed_config.package.name,
                &aws_cluster.ssh_user,
                &aws_cluster.ssh_private_key,
            )
            .await?;
            let _ = run_ray(
                RayCommand::Job(RayJob::Submit(Job {
                    working_dir,
                    command,
                })),
                &ray_config,
            )
            .await?;
            drop(child);
            drop(temp_dir);
        }
    };
    Ok(())
}

pub async fn handle_export(path: &Path, ray_config: RayConfig) -> anyhow::Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .await?;
    let contents =
        serde_yaml::to_string(&ray_config).expect("Serializing to string should always succeed");
    file.write_all(contents.as_bytes()).await?;
    println!(
        "Wrote the internally generated Ray YAML config file to the path: {}",
        style(format!("`{}`", path.display())).cyan(),
    );
    Ok(())
}

// helpers
// =============================================================================

const NOTEPAD_EMOJI: &str = "📝";
const CLOUD_EMOJI: &str = "🌥️";
const HAMMER_EMOJI: &str = "🔨";
const COMPUTER_EMOJI: &str = "💻";

async fn aws_ssh_helper(
    name: &str,
    ssh_user: &str,
    ssh_private_key: &Path,
) -> anyhow::Result<Child> {
    let instances = list_instances("us-west-2").await?;
    let mut addrs_and_instance_ids = instances
        .iter()
        .filter_map(|instance| {
            if instance.cluster_name_equals_ray_name(name) {
                instance
                    .public_ipv4_address
                    .map(|addr| (addr, instance.instance_id.clone()))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    let (addr, _) = match &*addrs_and_instance_ids {
        [] => anyhow::bail!("No clusters found"),
        [_] => addrs_and_instance_ids.pop().unwrap(),
        _ => {
            let options = addrs_and_instance_ids
                .iter()
                .map(|(addr, instance_id)| format!("{} (Instance ID: {})", addr, instance_id))
                .collect::<Vec<_>>();
            let (index, _) = with_selection_2(
                &format!("Multiple AWS clusters found with the name {}; which one do you want to connect into?", format!("`{}`", style(name).cyan())),
                &*options,
                &prefix(CLOUD_EMOJI),
            )?;
            addrs_and_instance_ids.swap_remove(index)
        }
    };
    let child = start_ssh_process(ssh_user, addr, ssh_private_key)?;
    Ok(child)
}

fn prefix(prefix: &str) -> ColorfulTheme {
    ColorfulTheme {
        prompt_prefix: style(prefix.into()),
        ..Default::default()
    }
}

fn add_instance_to_table(instance: &AwsInstance, table: &mut Table) {
    let status = instance.state.as_ref().map_or_else(
        || Cell::new("n/a").add_attribute(Attribute::Dim),
        |status| {
            let cell = Cell::new(status);
            match status {
                InstanceStateName::Running => cell.fg(Color::Green),
                InstanceStateName::Pending => cell.fg(Color::Yellow),
                InstanceStateName::ShuttingDown | InstanceStateName::Stopping => {
                    cell.fg(Color::DarkYellow)
                }
                InstanceStateName::Stopped | InstanceStateName::Terminated => cell.fg(Color::Red),
                _ => cell,
            }
        },
    );
    let ipv4 = instance
        .public_ipv4_address
        .as_ref()
        .map_or("n/a".into(), ToString::to_string);

    table.add_row(vec![
        Cell::new(instance.regular_name.to_string()).fg(Color::Cyan),
        Cell::new(&*instance.instance_id),
        status,
        Cell::new(ipv4),
    ]);
}

fn with_input<S: Into<String>>(
    prompt: &str,
    theme: &ColorfulTheme,
    default: S,
) -> anyhow::Result<StrRef> {
    let value = Input::<String>::with_theme(theme)
        .with_prompt(prompt)
        .default(default.into())
        .interact_text()?
        .into();
    Ok(value)
}

fn with_selection_2<'a>(
    prompt: &str,
    options: &'a [impl ToString],
    theme: &ColorfulTheme,
) -> anyhow::Result<(usize, &'a impl ToString)> {
    let selection_index = Select::with_theme(theme)
        .with_prompt(prompt)
        .default(0)
        .items(options)
        .interact()?;
    let selection = options
        .get(selection_index)
        .expect("Selection index should always be in bounds");
    Ok((selection_index, selection))
}

fn with_selection<T: Selectable>(prompt: &str, theme: &ColorfulTheme) -> anyhow::Result<T::Parsed> {
    let options = T::to_options();
    let selection = Select::with_theme(theme)
        .with_prompt(prompt)
        .default(0)
        .items(&options)
        .interact()?;
    let &selection = options
        .get(selection)
        .expect("Index should always be in bounds");
    T::parse(selection)
}

fn to_stylized_name_and_cloud(processed_config: &ProcessedConfig) -> (String, String) {
    let cloud_name = match processed_config.cluster.provider {
        processed::Provider::Aws(ref aws_cluster) => {
            format!("aws (region = {})", aws_cluster.region)
        }
    };

    let stylized_name = style(format!("`{}`", processed_config.package.name))
        .cyan()
        .to_string();
    let stylized_cloud = style(format!("`{}`", cloud_name)).cyan().to_string();

    (stylized_name, stylized_cloud)
}
