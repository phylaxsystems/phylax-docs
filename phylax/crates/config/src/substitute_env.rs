use std::{
    collections::HashMap,
    env,
    fmt::{self, Debug, Display},
    marker::PhantomData,
    str::FromStr,
};

use regex::Regex;
use serde::{
    de::{self},
    Deserialize, Deserializer, Serialize,
};

use crate::error::SubstError;
use serde::de::{DeserializeOwned, Visitor};

/// A wrapper type for deserializing environment variables or direct values.
#[derive(Debug, Serialize, Clone, Default, PartialEq)]
pub struct ValOrEnvVar<T: FromStr + Debug>(pub T);

impl<T: FromStr + Debug> ValOrEnvVar<T> {
    pub fn new(value: T) -> Self {
        ValOrEnvVar(value)
    }
}

impl<'de, T> Deserialize<'de> for ValOrEnvVar<T>
where
    T: DeserializeOwned + FromStr + Debug,
    T::Err: Debug,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct EnvOrVisitor<T> {
            marker: PhantomData<T>,
        }

        impl<'de, T> Visitor<'de> for EnvOrVisitor<T>
        where
            T: DeserializeOwned + FromStr + Debug,
            T::Err: Debug,
        {
            type Value = ValOrEnvVar<T>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter
                    .write_str("A string that can contain env variable ${KEY} or a direct value")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                let substituted =
                    validate_and_subst(&get_env_vars(), v).map_err(de::Error::custom)?;
                let parsed = substituted
                    .parse::<T>()
                    .map_err(|err| de::Error::custom(format!("{:?}", err)))?;
                Ok(ValOrEnvVar(parsed))
            }
        }

        deserializer.deserialize_string(EnvOrVisitor { marker: PhantomData })
    }
}

impl<T: FromStr> Display for ValOrEnvVar<T>
where
    T: Display + Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Get all the environment variables and return them
pub fn get_env_vars() -> HashMap<String, String> {
    let env_vars = env::vars();
    env_vars.collect()
}

/// Substitute anything that can be converted to a string with key/value from a hashmap.
///
/// # Arguments
///
/// * `map` - A reference to a HashMap where the key is a String and the value can be converted into
///   a String.
/// * `input` - A reference to an item that can be converted into a string.
///
/// # Returns
///
/// * A String after substitution.
pub fn subst<V: ToString + Debug>(map: &HashMap<String, V>, input: &str) -> String {
    let re = Regex::new(r"\$\{?([^\s}]+)\}?").expect("the regex definition is wrong");
    let result = re.replace_all(input, |caps: &regex::Captures| {
        map.get(&caps[1]).map_or_else(|| caps[0].to_string(), ToString::to_string)
    });
    result.to_string()
}

/// Check if the input string has malformed variable names.
///
/// # Arguments
///
/// * `input` - A reference to an item that can be converted into a string.
///
/// # Returns
///
/// * A Result which is Ok if no malformed variable names are found, otherwise Err with SubstError.
pub fn validate_subst(input: &str) -> Result<(), SubstError> {
    if input.is_empty() {
        return Err(SubstError::EmptyInput);
    }
    let re = Regex::new(r"\$\{?([^\s}]*)\}?").expect("wrong regex = bug");
    let result = re.captures_iter(input).find(|caps| {
        let key = &caps[0];
        (key.starts_with("${") && !key.ends_with('}')) || key == "${}" || key == "$"
    });
    match result {
        Some(caps) => {
            let key = &caps[0];
            Err(SubstError::MalformedKey(input.to_string(), key.to_string()))
        }
        None => Ok(()),
    }
}
/// Substitute and validate the input string.
///
/// # Arguments
///
/// * `map` - A reference to a HashMap where the key is a String and the value can be converted into
///   a String.
/// * `input` - A reference to an item that can be converted into a string.
///
/// # Returns
///
/// * A Result which is Ok if no malformed variable names are found and substitution is successful,
///   otherwise Err with SubstError.
pub fn validate_and_subst<V: ToString + Debug>(
    map: &HashMap<String, V>,
    input: &str,
) -> Result<String, SubstError> {
    validate_subst(input)?;
    Ok(subst(map, input))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use std::env;

    #[test]
    fn test_get_env_vars() {
        // Set an environment variable
        env::set_var("TEST", "test_value");
        let env_vars = get_env_vars();
        assert_eq!(env_vars.get("TEST"), Some(&"test_value".to_string()));
    }

    #[test]
    fn test_subst() {
        let mut map = HashMap::new();
        map.insert("key1".to_string(), "value1");
        map.insert("key2".to_string(), "value2");

        let input = "This is a ${key1} and this is a $key2 $key3";
        let expected_output = "This is a value1 and this is a value2 $key3";

        assert_eq!(subst(&map, input), expected_output);
    }
    #[test]
    fn test_subst_validity() {
        let input = "This is a ${key1} and this is a $key2";
        let result = validate_subst(input);
        assert!(result.is_ok());

        let input = "This is a ${} and this is a $key";
        let result = validate_subst(input);
        assert!(result.is_err());

        let input = "This is a ${key and this is a $key";
        let result = validate_subst(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_deserialize_direct_value() {
        let json = json!("42").to_string();
        let deserialized: Result<ValOrEnvVar<i32>, _> = serde_json::from_str(&json);
        assert!(deserialized.is_ok());
        assert_eq!(deserialized.unwrap().0, 42);
    }

    #[test]
    fn test_deserialize_env_var() {
        env::set_var("VALUE", "100");
        let json = json!("${VALUE}").to_string();
        let deserialized: Result<ValOrEnvVar<i32>, _> = serde_json::from_str(&json);
        assert!(deserialized.is_ok());
        assert_eq!(deserialized.unwrap().0, 100);
        env::remove_var("VALUE");
    }

    #[test]
    fn test_deserialize_invalid_env_var() {
        let json = json!("${NON_EXISTENT}").to_string();
        let deserialized: Result<ValOrEnvVar<i32>, _> = serde_json::from_str(&json);
        assert!(deserialized.is_err());
    }

    #[test]
    fn test_deserialize_malformed_env_var() {
        let json = json!("${").to_string();
        let deserialized: Result<ValOrEnvVar<i32>, _> = serde_json::from_str(&json);
        assert!(deserialized.is_err());
    }
}
