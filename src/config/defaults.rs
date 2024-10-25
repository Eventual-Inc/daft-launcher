use crate::StrRef;

fn base_setup_commands() -> Vec<String> {
    [
    "curl -LsSf https://astral.sh/uv/install.sh | sh",
    "uv python install {config.setup.python_version}",
    "uv python pin {config.setup.python_version}",
    "uv venv",
    r#"echo "alias pip='uv pip'" >> $HOME/.bashrc"#,
    r#"echo "source $HOME/.venv/bin/activate" >> $HOME/.bashrc"#,
    "source $HOME/.bashrc",
    r#"uv pip install "ray[default]=={config.setup.ray_version}" "getdaft" "deltalake""#,
].into_iter().map(ToString::to_string).collect()
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
