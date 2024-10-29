use crate::{
    aws::{
        assert_authenticated as assert_authenticated_with_aws,
        assert_non_clashing_cluster_name as assert_non_clashing_cluster_name_with_aws,
    },
    cli::{Down, Init},
    config::processed::{self, ProcessedConfig},
    utils::{assert_file_status, Status},
};

pub async fn assert_init(init: &Init) -> anyhow::Result<()> {
    assert_file_status(&init.name, Status::DoesNotExist).await?;
    Ok(())
}

pub async fn assert_up(processed_config: &ProcessedConfig) -> anyhow::Result<()> {
    assert_authenticated(Some(&processed_config.cluster.provider)).await?;
    assert_non_clashing_cluster_name(&processed_config).await?;
    Ok(())
}

pub async fn assert_down(down: &Down, processed_config: &ProcessedConfig) -> anyhow::Result<()> {
    match down.name.as_deref() {
        Some(..) => todo!(),
        None => assert_authenticated(Some(&processed_config.cluster.provider)).await?,
    };
    Ok(())
}

pub async fn assert_list() -> anyhow::Result<()> {
    assert_authenticated(None).await?;
    Ok(())
}

pub async fn assert_submit(processed_config: &ProcessedConfig) -> anyhow::Result<()> {
    assert_authenticated(Some(&processed_config.cluster.provider)).await?;
    Ok(())
}

// helpers
// =============================================================================

async fn assert_authenticated(provider: Option<&processed::Provider>) -> anyhow::Result<()> {
    let (authenticate_with_aws,) = provider.map_or((true,), |provider| match provider {
        processed::Provider::Aws(..) => (true,),
    });

    if authenticate_with_aws {
        assert_authenticated_with_aws().await?;
    };

    Ok(())
}

async fn assert_non_clashing_cluster_name(
    processed_config: &processed::ProcessedConfig,
) -> anyhow::Result<()> {
    match processed_config.cluster.provider {
        processed::Provider::Aws(ref aws_cluster) => {
            assert_non_clashing_cluster_name_with_aws(
                &processed_config.package.name,
                aws_cluster.region.to_string(),
            )
            .await?;
        }
    };
    Ok(())
}
