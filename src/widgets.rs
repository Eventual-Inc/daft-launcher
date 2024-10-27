use std::{borrow::Cow, time::Duration};

use console::style;
use indicatif::{ProgressBar, ProgressStyle};

const SUCCESS_EMOJI: &str = "✅";
const FAIL_EMOJI: &str = "❗️";

#[derive(Debug, Clone)]
pub struct Spinner {
    spinner: ProgressBar,
    message: Cow<'static, str>,
    terminated_successfully: bool,
}

impl Spinner {
    pub fn new(message: impl Into<Cow<'static, str>>) -> Self {
        let message = message.into();
        let spinner = ProgressBar::new_spinner();
        spinner.enable_steady_tick(Duration::from_millis(10));
        spinner.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.yellow}\t {msg}")
                .expect("Parsing style should always succeed"),
        );
        spinner.set_message(message.clone());
        Self {
            message,
            spinner,
            terminated_successfully: false,
        }
    }

    pub fn success(mut self) {
        self.terminated_successfully = true;
    }

    pub fn pause(&self, message: &str) {
        self.spinner
            .suspend(|| println!("{}", style(message).dim()));
    }
}

impl Drop for Spinner {
    fn drop(&mut self) {
        let emoji = if self.terminated_successfully {
            SUCCESS_EMOJI
        } else {
            FAIL_EMOJI
        };
        self.spinner.set_style(
            ProgressStyle::default_spinner()
                .template("{msg}")
                .expect("Parsing style should always succeed"),
        );
        self.spinner
            .finish_with_message(format!("{}\t{}", emoji, self.message));
    }
}
