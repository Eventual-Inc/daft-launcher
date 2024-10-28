//! Data definitions and helper functions for dealing with configuration files.
//!
//! # Overview
//!
//! The data serialization pipeline contains 3 stages:
//!
//! 1. `RawConfig`:
//! This is the basic, raw configuration values as directly read from the file.
//! *NO* higher-level invariants are enforced here. I.e., if we have an
//! invariant which states that the fields `a` and `b` must be mutually
//! exclusive, then `RawConfig` will represent them as:
//!
//! ```rs
//! struct RawConfig {
//!   a: Option<..>,
//!   b: Option<..>,
//! }
//! ```
//!
//! (The above is an illegal representation of this invariant since both `a` and
//! `b` can both be `Some(..)`). However, this stage only cares about parsing,
//! and thus, invariants are not enforced here just yet.
//!
//! 2. `ProcessedConfig`:
//! This is the stage when the data has been processed and the higher-level
//! invariants will be enforced. This is the "good" data format. For example,
//! if the fields `a` and `b` are supposed to be mutually exclusive, then
//! `ProcessedConfig` will represent them as:
//!
//! ```rs
//! struct RawConfig {
//!   a_xor_b: either::Either<..>,
//! }
//! ```
//!
//! This will prevent *both* `a` and `b` from being `Some(..)` at the same time,
//! thus upholding our invariant guarantees.
//!
//! Converting from a `RawConfig` to a `ProcessedConfig` is a fallible
//! operation.
//!
//! 3. `RayConfig`:
//! A `RayConfig` is just a simple schema mapping from our custom,
//! `daft-launcher` toml schema to `Ray`'s yaml schema. Nothing intelligent is
//! going on here; it's just a plain schema structure mapping. As a result, this
//! is a simple, infallible conversion.

pub mod defaults;
pub mod processed;
pub mod raw;
pub mod ray;

use std::{fs::OpenOptions, io::Read, path::Path, sync::LazyLock};

use anyhow::Context;
use processed::ProcessedConfig;
use semver::Version;

use crate::{
    config::{raw::RawConfig, ray::RayConfig},
    PathRef,
};

static DAFT_LAUNCHER_VERSION: LazyLock<Version> =
    LazyLock::new(|| env!("CARGO_PKG_VERSION").parse().unwrap());

pub trait Selectable {
    type Parsed;

    fn to_options() -> &'static [&'static str];
    fn parse(s: &str) -> anyhow::Result<Self::Parsed>;
}

/// The main entry-point into working with `daft-launcher` based configuration files.
///
/// Reads the contents at the given path and attempts to deserialize it into a
/// `ProcessedConfig` (in which schema and high-level data invariants are
/// enforced). Upon successful deserialization, the `ProcessedConfig` is used to
/// generate a `RayConfig` counterpart. Finally, both the `ProcessedConfig` and
/// `RayConfig` are both returned.
pub fn read(path: &Path) -> anyhow::Result<(ProcessedConfig, RayConfig)> {
    let mut file =
        OpenOptions::new().read(true).open(path).with_context(|| {
            format!("No configuration file found at the path `{}`; please run `daft init-config` to generate a configuration file", path.display())
        })?;
    let mut buf = String::new();
    let _ = file
        .read_to_string(&mut buf)
        .with_context(|| format!("Failed to read file {path:?}"))?;
    let raw_config: RawConfig = toml::from_str(&buf)?;
    let processed_config: ProcessedConfig = raw_config.try_into()?;
    if !processed_config
        .package
        .daft_launcher_version
        .matches(&*DAFT_LAUNCHER_VERSION)
    {
        anyhow::bail!(
            "The version requirement in the config file located at {:?} (version-requirement {}) is not satisfied by this binary's version (version {})",
            path.display(),
            processed_config.package.daft_launcher_version,
            &*DAFT_LAUNCHER_VERSION,
        );
    }
    let ray_config: RayConfig = processed_config.clone().into();
    Ok((processed_config, ray_config))
}
