use aws_sdk_ec2::types::InstanceStateName;
use comfy_table::{
    modifiers::{UTF8_ROUND_CORNERS, UTF8_SOLID_INNER_BORDERS},
    presets::UTF8_FULL,
    Attribute, Cell, CellAlignment, Color, ContentArrangement, Table,
};
use console::style;
use dialoguer::{theme::ColorfulTheme, Input, Select};
use tokio::io::AsyncWriteExt;

use crate::{
    aws::{list_instances, AwsInstance},
    cli::{Init, List},
    config::{
        defaults::{normal_image_id, normal_instance_type},
        processed::{self, ProcessedConfig},
        raw::{
            default_name, AwsCluster, AwsCustomType, AwsTemplateType, Cluster, Package, Provider,
            RawConfig,
        },
        ray::RayConfig,
        Selectable,
    },
    ray::{run_ray, RaySubcommand},
    utils::create_new_file,
    StrRef,
};

const NOTEPAD_EMOJI: &str = "📝";
const CLOUD_EMOJI: &str = "🌥️";
const HAMMER_EMOJI: &str = "🔨";
const COMPUTER_EMOJI: &str = "💻";

fn prefix(prefix: &str) -> ColorfulTheme {
    ColorfulTheme {
        prompt_prefix: style(prefix.into()),
        ..Default::default()
    }
}

pub async fn handle_init(init: Init) -> anyhow::Result<()> {
    let raw_config = if init.default {
        RawConfig::default()
    } else {
        let name = with_input("Cluster name", &prefix(NOTEPAD_EMOJI), default_name())?;
        let provider = match with_selections::<Provider>("Cloud provider", &prefix(CLOUD_EMOJI))? {
            Provider::Aws(aws_cluster) => {
                let template =
                    with_selections::<AwsTemplateType>("Template", &prefix(HAMMER_EMOJI))?;
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
        "Created file at: {}",
        style(format!("`{}`", init.name.display())).cyan(),
    );
    Ok(())
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

fn with_selections<T: Selectable>(
    prompt: &str,
    theme: &ColorfulTheme,
) -> anyhow::Result<T::Parsed> {
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

pub async fn handle_up(
    processed_config: ProcessedConfig,
    ray_config: RayConfig,
) -> anyhow::Result<()> {
    let cloud_name = match processed_config.cluster.provider {
        processed::Provider::Aws(ref aws_cluster) => {
            if aws_cluster.iam_instance_profile_arn.is_none() {
                log::warn!("You specified no IAM instance profile ARN; this may cause limit your cluster's abilities to interface with auxiliary AWS services");
            }
            format!("`aws (region = {})`", aws_cluster.region)
        }
    };
    run_ray(&ray_config, RaySubcommand::Up, &[]).await?;
    println!(
        "Successfully spun up the cluster {} in your {} cloud",
        style(format!("`{}`", processed_config.package.name)).cyan(),
        style(format!("`{}`", cloud_name)).cyan(),
    );
    Ok(())
}

pub async fn handle_down(ray_config: RayConfig) -> anyhow::Result<()> {
    run_ray(&ray_config, RaySubcommand::Down, &[]).await
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
                    InstanceStateName::Stopped | InstanceStateName::Terminated => {
                        cell.fg(Color::Red)
                    }
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

    for instance in &instances {
        let is_running = instance
            .state
            .as_ref()
            .map_or(false, |state| *state == InstanceStateName::Running);
        let running_condition = !(list.running && !is_running);
        let regex_condition = list
            .name
            .as_ref()
            .map_or(true, |regex| regex.is_match(&instance.regular_name));
        if running_condition && regex_condition {
            add_instance_to_table(instance, &mut table);
        }
    }

    println!("{}", table);

    Ok(())
}
