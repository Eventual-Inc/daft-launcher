#[cfg(test)]
mod tests;

use std::ops::{Deref, DerefMut};

use serde::{Deserialize, Deserializer};

#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessableOption<T> {
    RawNone,
    RawSome(T),
    Processed(T),
}

impl<T: Default> Default for ProcessableOption<T> {
    fn default() -> Self {
        Self::RawSome(T::default())
    }
}

impl<T> From<Option<T>> for ProcessableOption<T> {
    fn from(value: Option<T>) -> Self {
        Self::new(value)
    }
}

impl<T> From<ProcessableOption<T>> for Option<T> {
    fn from(value: ProcessableOption<T>) -> Self {
        match value {
            ProcessableOption::RawNone => None,
            ProcessableOption::RawSome(t) | ProcessableOption::Processed(t) => {
                Some(t)
            }
        }
    }
}

impl<T> ProcessableOption<T> {
    pub fn empty() -> Self {
        Self::RawNone
    }

    pub fn nonempty(t: T) -> Self {
        Self::RawSome(t)
    }

    pub fn is_processed(&self) -> bool {
        matches!(self, Self::Processed(..))
    }

    pub fn new(t: Option<T>) -> Self {
        match t {
            Some(t) => Self::RawSome(t),
            None => Self::RawNone,
        }
    }

    pub fn as_ref(&self) -> ProcessableOption<&T> {
        match self {
            Self::RawNone => ProcessableOption::RawNone,
            Self::RawSome(t) => ProcessableOption::RawSome(t),
            Self::Processed(t) => ProcessableOption::Processed(t),
        }
    }

    pub fn as_mut(&mut self) -> ProcessableOption<&mut T> {
        match self {
            Self::RawNone => ProcessableOption::RawNone,
            Self::RawSome(t) => ProcessableOption::RawSome(t),
            Self::Processed(t) => ProcessableOption::Processed(t),
        }
    }

    pub fn to_processed(self) -> ProcessableOption<Option<T>> {
        match self {
            Self::RawNone => ProcessableOption::Processed(None),
            Self::RawSome(t) | Self::Processed(t) => {
                ProcessableOption::Processed(Some(t))
            }
        }
    }

    pub fn map<F, U>(self, f: F) -> ProcessableOption<U>
    where
        F: FnOnce(Option<T>) -> U,
    {
        match self {
            Self::RawNone => ProcessableOption::Processed(f(None)),
            Self::RawSome(t) => ProcessableOption::Processed(f(Some(t))),
            Self::Processed(..) => panic!(),
        }
    }

    pub fn idemmap<F>(self, f: F) -> Self
    where
        F: FnOnce(Option<T>) -> T,
    {
        match self {
            Self::RawNone => ProcessableOption::Processed(f(None)),
            Self::RawSome(t) => ProcessableOption::Processed(f(Some(t))),
            processed @ Self::Processed(..) => processed,
        }
    }

    pub fn try_map<F, U, Err>(self, f: F) -> Result<ProcessableOption<U>, Err>
    where
        F: FnOnce(Option<T>) -> Result<U, Err>,
    {
        Ok(match self {
            Self::RawNone => ProcessableOption::Processed(f(None)?),
            Self::RawSome(t) => ProcessableOption::Processed(f(Some(t))?),
            Self::Processed(..) => panic!(),
        })
    }

    pub fn try_idemmap<F, Err>(self, f: F) -> Result<Self, Err>
    where
        F: FnOnce(Option<T>) -> Result<T, Err>,
    {
        Ok(match self {
            Self::RawNone => ProcessableOption::Processed(f(None)?),
            Self::RawSome(t) => ProcessableOption::Processed(f(Some(t))?),
            processed @ Self::Processed(..) => processed,
        })
    }

    pub fn idemmap_some<F>(self, f: F) -> Self
    where
        F: FnOnce() -> T,
    {
        match self {
            Self::RawNone => Self::Processed(f()),
            Self::RawSome(t) => Self::Processed(t),
            processed @ Self::Processed(..) => processed,
        }
    }

    pub fn try_idemmap_some<F, Err>(self, f: F) -> Result<Self, Err>
    where
        F: FnOnce() -> Result<T, Err>,
    {
        Ok(match self {
            Self::RawNone => Self::Processed(f()?),
            Self::RawSome(t) => Self::Processed(t),
            processed @ Self::Processed(..) => processed,
        })
    }

    pub fn as_processed(&self) -> &T {
        match self {
            Self::RawSome(..) | Self::RawNone => panic!(),
            Self::Processed(t) => t,
        }
    }

    pub fn as_processed_mut(&mut self) -> &mut T {
        match self {
            Self::RawSome(..) | Self::RawNone => panic!(),
            Self::Processed(t) => t,
        }
    }
}

impl<T> Deref for ProcessableOption<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.as_processed()
    }
}

impl<T> DerefMut for ProcessableOption<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_processed_mut()
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for ProcessableOption<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let t = Option::deserialize(deserializer)?;
        Ok(Self::new(t))
    }
}
