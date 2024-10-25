use std::borrow::Cow;

use crate::{config::processed::Package, StrRef};

pub fn base_setup_commands(package: &Package) -> Vec<StrRef> {
    [
        "curl -LsSf https://astral.sh/uv/install.sh | sh".into(),
        format!("uv python install {}", package.python_version).into(),
        format!("uv python pin {}", package.python_version).into(),
        "uv v".into(),
        r#"echo "alias pip='uv pip'" >> $HOME/.bashrc"#.into(),
        r#"echo "source $HOME/.venv/bin/activate" >> $HOME/.bashrc"#.into(),
        "source $HOME/.bashrc".into(),
        format!(
            r#"uv pip install "ray[default]=={}" "getdaft" "deltalake""#,
            package.ray_version
        )
        .into(),
    ]
    .into_iter()
    .map(|command: Cow<_>| command.as_ref().into())
    .collect()
}

pub fn default_region() -> StrRef {
    "us-west-2".into()
}

pub fn default_ssh_user() -> StrRef {
    "ec2-user".into()
}

pub fn light_image_id() -> StrRef {
    "ami-07c5ecd8498c59db5".into()
}

pub fn light_instance_type() -> StrRef {
    "t2.nano".into()
}

pub fn normal_image_id() -> StrRef {
    "ami-07dcfc8123b5479a8".into()
}

pub fn normal_instance_type() -> StrRef {
    "m7g.medium".into()
}

pub const DEFAULT_NUMBER_OF_WORKERS: usize = 2;
