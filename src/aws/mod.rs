//! A module for interacting with all AWS service abstractions provided by us.
//!
//! # Note
//! All public functions in this module should be wrapped by the `spinner`
//! macro! These functions should *NOT* perform any business logic directly, but
//! instead just act as a wrapper/proxy. All actual internal implementations
//! should be placed inside of the (private) [`_impl`] module.

mod _impl;

use std::borrow::Cow;

pub use _impl::AwsInstance;

pub async fn assert_authenticated() -> anyhow::Result<()> {
    spinner! {
        "Authenticating with AWS!",
        {
            _impl::assert_authenticated().await?;
            Ok(())
        },
    }
}

pub async fn list_instances(
    region: impl Into<Cow<'static, str>>,
) -> anyhow::Result<Vec<AwsInstance>> {
    let region: Cow<_> = region.into();
    spinner! {
        format!(r#"Grabbing all AWS EC2 instances in the "{}" region"#, &region),
        {
            let instances = _impl::list_instances(region).await?;
            Ok(instances)
        },
    }
}

pub async fn assert_non_clashing_cluster_name(
    name: impl AsRef<str>,
    region: impl Into<Cow<'static, str>>,
) -> anyhow::Result<()> {
    spinner! {
        "Checking if the cluster name is unique",
        {
            let name = name.as_ref();
            let region = region.into();
            _impl::assert_non_clashing_cluster_name(name, region).await?;
            Ok(())
        },
    }
}
