mod _impl;

use std::{net::Ipv4Addr, path::Path};

use console::style;

pub use _impl::{
    assert_executable_exists, assert_file_status, create_new_file, create_temporary_ray_file,
    expand, path_to_str, Status,
};
use tokio::process::Child;

pub fn start_ssh_process(
    user: &str,
    addr: Ipv4Addr,
    ssh_private_key: &Path,
) -> anyhow::Result<Child> {
    spinner! {
        format!(
            "Establishing an ssh connection to address {} for the user {} with the identity key located at {}",
            style(addr).cyan(),
            style(user).cyan(),
            style(format!("`{}`", ssh_private_key.display())).cyan(),
        ),
        {
            let child = _impl::start_ssh_process(user, addr, ssh_private_key)?;
            Ok(child)
        },
    }
}
