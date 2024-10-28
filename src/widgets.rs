//! A "widgets" module which exposes high-level, UI primitives to be displayed
//! on the CLI.
//!
//! # Examples
//! A simple example of what a widget is is the [`Spinner`].
//! The [`Spinner`] is a continuously spinning set of characters that are
//! rendered (in the same position) on the CLI. While some arbitrary
//! computations are being performed, the spinner is in a "spinning" state. As
//! soon as the computation is done, the spinner can be triggered to display a
//! success or failure notification on the CLI.

use std::{borrow::Cow, time::Duration};

use console::style;
use indicatif::{ProgressBar, ProgressStyle};

const SUCCESS_EMOJI: &str = "✅";
const FAIL_EMOJI: &str = "❗️";

/// A spinning indicator that is displayed on the command line.
///
/// # Note
/// [`Spinner`] implements the [`Drop`] trait, for which once dropped, it will
/// automatically display a failure message. Defaulting to displaying an error
/// message upon being dropped is convenient, especially in functions which
/// perform fallible operations that fail quickly (thus, the function returns
/// early and the [`Spinner`] is dropped, thus triggering a failure message).
///
/// If you want to explicitly state that the computation was successful,
/// explicitly call the [`Spinner::success`] method. This will cause the
/// [`Spinner`] to instead display a success message upon being dropped.
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

    /// Indicate to the [`Spinner`] that the computation was successful.
    ///
    /// As a result, when this [`Spinner`] is dropped, it will display a success
    /// message (instead of the default failure message).
    pub fn success(mut self) {
        self.terminated_successfully = true;
    }

    /// Pause the [`Spinner`] rendering loop in order to print a message to the
    /// CLI.
    ///
    /// # Note
    /// 1. This API will print the message using a "dimmed" colour.
    ///
    /// 2. If you naively just print to the CLI without pausing the spinner, a
    /// weird rendering artefact will occur. The [`Spinner`]'s will start
    /// printing to the next line, and the previous line will contain a frozen
    /// version of the last spinning state. This is a result of the
    /// limited rendering APIs of terminal-based UIs.
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
