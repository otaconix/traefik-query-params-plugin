use std::fmt::Debug;

use grep_matcher::Matcher;
use grep_regex::RegexMatcher;
use serde::Deserialize;
use serde::Deserializer;
use serde_aux::field_attributes::deserialize_number_from_string;

use crate::util::position_to_index;

#[derive(Deserialize, PartialEq, Debug)]
pub struct RemoveOperation {
    #[serde(
        deserialize_with = "RemoveOperation::deserialize",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub(crate) position: Option<i8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) regexp: Option<Regex>,
}

pub struct Regex {
    regexp: String,
    pub(crate) matcher: RegexMatcher,
}

impl Regex {
    fn new(regexp: String) -> Result<Self, grep_regex::Error> {
        Ok(Self {
            matcher: RegexMatcher::new(&regexp)?,
            regexp,
        })
    }
}

impl Debug for Regex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.regexp.fmt(f)
    }
}

impl<'de> Deserialize<'de> for Regex {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Regex::new(String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

impl PartialEq for Regex {
    fn eq(&self, other: &Self) -> bool {
        self.regexp.eq(&other.regexp)
    }
}

impl RemoveOperation {
    pub fn apply(&self, name: &str, query: &mut Vec<(String, String)>) {
        match self.position {
            None => query.retain(|(key, value)| {
                key != name
                    && self
                        .regexp
                        .as_ref()
                        .map(|re| re.matcher.is_match(value.as_bytes()).unwrap())
                        .unwrap_or(true)
            }),
            Some(position) => {
                let matching_param_indices = query
                    .iter()
                    .enumerate()
                    .filter(|(_, (key, _))| key == name)
                    .map(|(index, _)| index)
                    .collect::<Vec<_>>();
                if !matching_param_indices.is_empty() {
                    let index_to_remove =
                        position_to_index(position, &matching_param_indices, false);
                    query.remove(index_to_remove);
                }
            }
        }
    }

    #[cfg(test)]
    pub fn all() -> Self {
        Self {
            position: None,
            regexp: None,
        }
    }

    #[cfg(test)]
    pub fn position(position: i8) -> Self {
        Self {
            position: Some(position),
            regexp: None,
        }
    }

    #[cfg(test)]
    pub fn matching_regexp<T: ToString + ?Sized>(
        self,
        regexp: &T,
    ) -> Result<Self, grep_regex::Error> {
        Ok(Self {
            regexp: Some(Regex::new(regexp.to_string())?),
            ..self
        })
    }

    fn deserialize<'de, D>(deserializer: D) -> Result<Option<i8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Internal {
            #[serde(deserialize_with = "deserialize_number_from_string")]
            Num(i8),
            Nothing,
        }

        match Internal::deserialize(deserializer)? {
            Internal::Num(n) => Ok(Some(n)),
            Internal::Nothing => Ok(None),
        }
    }
}
