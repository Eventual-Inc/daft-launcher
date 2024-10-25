use std::{borrow::Cow, time::Duration};

use console::style;
use indicatif::{ProgressBar, ProgressStyle};

const SUCCESS_EMOJI: &str = "✅";
const FAIL_EMOJI: &str = "❗️";

#[derive(Debug, Clone)]
pub struct Spinner {
    spinner: ProgressBar,
    message: Cow<'static, str>,
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
        Self { message, spinner }
    }

    pub fn success(self) {
        self.finish(SUCCESS_EMOJI)
    }

    pub fn fail(self) {
        self.finish(FAIL_EMOJI)
    }

    fn finish(&self, prefix: &str) {
        self.spinner.set_style(
            ProgressStyle::default_spinner()
                .template("{msg}")
                .expect("Parsing style should always succeed"),
        );
        self.spinner
            .finish_with_message(format!("{}\t{}", prefix, self.message));
    }

    pub fn pause(&self, message: &str) {
        self.spinner
            .suspend(|| println!("{}", style(message).dim()));
    }
}
