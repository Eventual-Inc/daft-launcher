use serde::{Deserialize, Deserializer};

#[must_use]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessableOption<T> {
    RawNone,
    RawSome(Option<T>),
    Processed(T),
}

impl<T: Default> Default for ProcessableOption<T> {
    fn default() -> Self {
        Self::RawSome(Some(T::default()))
    }
}

impl<T> ProcessableOption<T> {
    fn yank(&mut self) -> T {
        match self {
            Self::RawSome(t) => t.take().expect(
                "This variant MUST always be of type `Self::RawSome(Some(..))`",
            ),
            _ => panic!(),
        }
    }

    pub fn new(t: Option<T>) -> Self {
        match t {
            Some(t) => Self::RawSome(Some(t)),
            None => Self::RawNone,
        }
    }

    pub fn try_process<F>(&mut self, f: F) -> anyhow::Result<()>
    where
        F: FnOnce(Option<T>) -> anyhow::Result<T>,
    {
        match self {
            Self::RawNone => *self = Self::Processed(f(None)?),
            Self::RawSome(..) => {
                let t = self.yank();
                *self = Self::Processed(f(Some(t))?)
            }
            Self::Processed(..) => panic!(),
        };
        Ok(())
    }

    pub fn or_else_try_process<F>(&mut self, f: F) -> anyhow::Result<()>
    where
        F: FnOnce() -> anyhow::Result<T>,
    {
        match self {
            Self::RawNone => *self = Self::Processed(f()?),
            Self::RawSome(..) => {
                let t = self.yank();
                *self = Self::Processed(t);
            }
            Self::Processed(..) => panic!(),
        };
        Ok(())
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

impl<'de, T: Deserialize<'de>> Deserialize<'de> for ProcessableOption<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let t = Option::deserialize(deserializer)?;
        Ok(Self::new(t))
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::ProcessableOption;

    #[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
    struct Config {
        flag: ProcessableOption<bool>,
    }

    #[test]
    fn test_deser_with_no_value() {
        let result = toml::from_str("");
        assert_eq!(
            result,
            Ok(Config {
                flag: ProcessableOption::RawNone
            })
        );
    }

    #[test]
    fn test_deser_with_value() {
        let result = toml::from_str("flag = true");
        assert_eq!(
            result,
            Ok(Config {
                flag: ProcessableOption::RawSome(Some(true))
            })
        );
    }

    #[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
    struct ConfigWithNestedOption {
        flag: ProcessableOption<Option<bool>>,
    }

    #[test]
    fn test_deser_with_nested_option_with_no_value() {
        let result = toml::from_str("");
        assert_eq!(
            result,
            Ok(ConfigWithNestedOption {
                flag: ProcessableOption::RawNone
            })
        );
    }

    #[test]
    fn test_deser_with_nested_option_with_value() {
        let result = toml::from_str("flag = true");
        assert_eq!(
            result,
            Ok(ConfigWithNestedOption {
                flag: ProcessableOption::RawSome(Some(Some(true)))
            })
        );
    }

    #[test]
    fn test() {
        let mut x = ProcessableOption::new(Some(0u8));
    }
}
