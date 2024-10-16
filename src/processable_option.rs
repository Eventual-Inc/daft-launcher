use std::convert::identity;

use serde::{Deserialize, Deserializer};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessableOption<T> {
    Raw(Option<T>),
    Processed(T),
}

impl<T> Default for ProcessableOption<T> {
    fn default() -> Self {
        Self::Raw(None)
    }
}

impl<T> ProcessableOption<T> {
    pub fn or_else<F>(self, default: F) -> Self
    where
        F: FnOnce() -> T,
    {
        self.map_or_else(default, identity)
    }

    pub fn map_or_else<Fd, F, U>(
        self,
        default: Fd,
        f: F,
    ) -> ProcessableOption<U>
    where
        Fd: FnOnce() -> U,
        F: FnOnce(T) -> U,
    {
        match self {
            Self::Raw(Some(t)) => ProcessableOption::Processed(f(t)),
            Self::Raw(None) => ProcessableOption::Processed(default()),
            Self::Processed(t) => ProcessableOption::Processed(f(t)),
        }
    }
}

impl<T, E> ProcessableOption<Result<T, E>> {
    pub fn transpose(self) -> Result<ProcessableOption<T>, E> {
        match self {
            Self::Raw(t) => Ok(ProcessableOption::Raw(t.transpose()?)),
            Self::Processed(t) => Ok(ProcessableOption::Processed(t?)),
        }
    }
}

impl<T> ProcessableOption<Option<T>> {
    pub fn transpose(self) -> Option<ProcessableOption<T>> {
        match self {
            Self::Raw(t) => Some(ProcessableOption::Raw(t?)),
            Self::Processed(t) => Some(ProcessableOption::Processed(t?)),
        }
    }
}

impl<'de, T> Deserialize<'de> for ProcessableOption<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Option::deserialize(deserializer).map(Self::Raw)
    }
}
