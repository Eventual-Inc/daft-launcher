use either::Either;
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
pub struct Processable<R, P>(Either<R, P>);

impl<R: Default, P> Default for Processable<R, P> {
    fn default() -> Self {
        Self(Either::Left(R::default()))
    }
}

impl<R, P> Processable<R, P> {
    pub fn process_state(&self) -> ProcessState {
        match self {
            Self(Either::Left(_)) => ProcessState::Raw,
            Self(Either::Right(_)) => ProcessState::Processed,
        }
    }

    pub fn is_processed(&self) -> bool {
        self.process_state().is_processed()
    }

    pub fn new(r: R) -> Self {
        Self(Either::Left(r))
    }

    pub fn process<F>(self, f: F) -> Self
    where
        F: FnOnce(R) -> P,
    {
        match self.0 {
            Either::Left(r) => Self(Either::Right(f(r))),
            Either::Right(p) => Self(Either::Right(p)),
        }
    }

    pub fn try_process<F, E>(self, f: F) -> Result<Self, E>
    where
        F: FnOnce(R) -> Result<P, E>,
    {
        Ok(match self.0 {
            Either::Left(r) => Self(Either::Right(f(r)?)),
            Either::Right(p) => Self(Either::Right(p)),
        })
    }

    pub fn into_processed(self) -> P {
        match self.0 {
            Either::Left(_) => panic!(),
            Either::Right(p) => p,
        }
    }
}

pub type ProcessableOption<R> = Processable<Option<R>, R>;

impl<R> ProcessableOption<R> {
    pub fn or_else<F>(self, default: F) -> Self
    where
        F: FnOnce() -> R,
    {
        Self(Either::Right(match self.0 {
            Either::Left(None) => default(),
            Either::Left(Some(p)) | Either::Right(p) => p,
        }))
    }

    pub fn try_or_else<F, Err>(self, default: F) -> Result<Self, Err>
    where
        F: FnOnce() -> Result<R, Err>,
    {
        Ok(Self(Either::Right(match self.0 {
            Either::Left(None) => default()?,
            Either::Left(Some(p)) | Either::Right(p) => p,
        })))
    }
}

impl<'de, R, P> Deserialize<'de> for Processable<R, P>
where
    R: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        R::deserialize(deserializer).map(|r| Self(Either::Left(r)))
    }
}
