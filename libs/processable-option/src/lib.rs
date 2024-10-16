use serde::{Deserialize, Deserializer};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessState {
    Raw,
    Processed,
}

impl ProcessState {
    pub fn is_processed(&self) -> bool {
        matches!(self, Self::Processed)
    }
}

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
    pub fn new(t: Option<T>) -> Self {
        Self::Raw(t)
    }

    pub fn from_option_and_state(
        t: Option<T>,
        process_state: ProcessState,
    ) -> Self {
        match (t, process_state) {
            (Some(t), ProcessState::Processed) => Self::Processed(t),
            (Some(t), ProcessState::Raw) => Self::Raw(Some(t)),
            (None, ProcessState::Processed) => panic!(),
            (None, ProcessState::Raw) => Self::Raw(None),
        }
    }

    pub fn to_option_and_state(self) -> (Option<T>, ProcessState) {
        match self {
            Self::Raw(t) => (t, ProcessState::Raw),
            Self::Processed(t) => (Some(t), ProcessState::Processed),
        }
    }

    pub fn process_state(&self) -> ProcessState {
        match self {
            Self::Processed(..) => ProcessState::Processed,
            Self::Raw(..) => ProcessState::Raw,
        }
    }

    pub fn is_processed(&self) -> bool {
        self.process_state().is_processed()
    }

    pub fn process<F>(self, default: F) -> Self
    where
        F: FnOnce() -> T,
    {
        match self {
            Self::Raw(Some(t)) => Self::Processed(t),
            Self::Raw(None) => Self::Processed(default()),
            Self::Processed(t) => Self::Processed(t),
        }
    }

    pub fn try_process<F, Err>(self, default: F) -> Result<Self, Err>
    where
        F: FnOnce() -> Result<T, Err>,
    {
        Ok(match self {
            Self::Raw(Some(t)) => Self::Processed(t),
            Self::Raw(None) => Self::Processed(default()?),
            Self::Processed(t) => Self::Processed(t),
        })
    }

    pub fn as_processed(&self) -> &T {
        match self {
            Self::Raw(..) => panic!(),
            Self::Processed(t) => t,
        }
    }

    pub fn into_processed(self) -> T {
        match self {
            Self::Raw(..) => panic!(),
            Self::Processed(t) => t,
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

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(Some(0), || 1, 0)]
    #[case(None, || 1, 1)]
    fn test_processing(
        #[case] value: Option<u8>,
        #[case] default: fn() -> u8,
        #[case] expected: u8,
    ) {
        let manual = {
            let processable_option = ProcessableOption::new(value.clone());
            let (option, _) = processable_option.to_option_and_state();
            let option = option.or_else(|| Some(default()));
            ProcessableOption::from_option_and_state(
                option,
                ProcessState::Processed,
            )
        };
        let actual = ProcessableOption::new(value).process(default);
        assert_eq!(manual, actual);
        assert_eq!(
            actual,
            ProcessableOption::from_option_and_state(
                Some(expected),
                ProcessState::Processed,
            )
        );
    }
}
