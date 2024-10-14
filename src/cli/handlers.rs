use std::fs;
use std::io;
use std::io::Write;

use crate::cli;

const DEFAULT_CONFIG: &str = include_str!(path_from_root!("assets" / "default.toml"));

pub fn handle_init_config(init_config: cli::InitConfig) -> crate::Result<()> {
    if init_config.interactive {
        todo!()
    } else {
        let mut file = match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&init_config.name)
        {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                return Err(crate::Error::AlreadyExistsError(init_config.name));
            }
            err => err?,
        };
        file.write_all(DEFAULT_CONFIG.as_bytes())?;
        Ok(())
    }
}

pub fn handle_up(up: cli::Up) -> crate::Result<()> {
    todo!()
}

pub fn handle_down(down: cli::Down) -> crate::Result<()> {
    todo!()
}

pub fn handle_submit(submit: cli::Submit) -> crate::Result<()> {
    todo!()
}

pub fn handle_connect(connect: cli::Connect) -> crate::Result<()> {
    todo!()
}

pub fn handle_dashboard(dashboard: cli::Dashboard) -> crate::Result<()> {
    todo!()
}

pub fn handle_sql(sql: cli::Sql) -> crate::Result<()> {
    todo!()
}
