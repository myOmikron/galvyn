//! [`std`] implementations for [`Redacted`]

use std::cmp::Ordering;
use std::fmt;
use std::fmt::Formatter;
use std::hash::Hash;
use std::hash::Hasher;
use std::marker::PhantomData;

use crate::misc::redacted::Redacted;
use crate::misc::redacted::config::PartToRedact;
use crate::misc::redacted::config::RedactionConfig;

impl<T, Config> fmt::Debug for Redacted<T, Config>
where
    T: fmt::Debug,
    Config: RedactionConfig,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let part_to_redact = Config::VALUE.debug;

        if matches!(part_to_redact, PartToRedact::Everything) {
            f.write_str("-redacted-")
        } else {
            let value = format!("{:?}", self.sensitiv);
            f.write_str(part_to_redact.redact(&value).as_ref())
        }
    }
}

impl<T, Config> fmt::Display for Redacted<T, Config>
where
    T: fmt::Display,
    Config: RedactionConfig,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let part_to_redact = Config::VALUE.display;

        if matches!(part_to_redact, PartToRedact::Everything) {
            f.write_str("-redacted-")
        } else {
            let value = format!("{}", self.sensitiv);
            f.write_str(part_to_redact.redact(&value).as_ref())
        }
    }
}

impl<T, Config> Copy for Redacted<T, Config> where T: Copy {}
impl<T, Config> Clone for Redacted<T, Config>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        Self {
            sensitiv: self.sensitiv.clone(),
            config: PhantomData,
        }
    }

    fn clone_from(&mut self, source: &Self) {
        self.sensitiv.clone_from(&source.sensitiv)
    }
}
impl<T, Config> Eq for Redacted<T, Config> where T: Eq {}
impl<T, Config> PartialEq for Redacted<T, Config>
where
    T: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.sensitiv.eq(&other.sensitiv)
    }
}
impl<T, Config> Ord for Redacted<T, Config>
where
    T: Ord,
{
    fn cmp(&self, other: &Self) -> Ordering {
        self.sensitiv.cmp(&other.sensitiv)
    }
}
impl<T, Config> PartialOrd for Redacted<T, Config>
where
    T: PartialOrd,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.sensitiv.partial_cmp(&other.sensitiv)
    }
}
impl<T, Config> Hash for Redacted<T, Config>
where
    T: Hash,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.sensitiv.hash(state)
    }
}
