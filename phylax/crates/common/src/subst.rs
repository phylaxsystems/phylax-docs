use regex::Regex;
use std::{
    collections::HashMap,
    fmt::{Debug, Display},
};
use thiserror::Error;

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
pub fn subst<T: ToString + Display, V: ToString + Debug>(
    map: &HashMap<String, V>,
    input: &T,
) -> String {
    let input_string = input.to_string();
    let re = Regex::new(r"\$\{?([^\s}]+)\}?").expect("the regex definition is wrong");
    let result = re.replace_all(&input_string, |caps: &regex::Captures| {
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
pub fn validate_subst<T: ToString + Display>(input: &T) -> Result<(), SubstError> {
    let input_string = input.to_string();
    if input_string.is_empty() {
        return Err(SubstError::EmptyInput);
    }
    let re = Regex::new(r"\$\{?([^\s}]*)\}?").expect("wrong regex = bug");
    let result = re.captures_iter(&input_string).find(|caps| {
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
pub fn subst_and_validate<T: ToString + Display, V: ToString + Debug>(
    map: &HashMap<String, V>,
    input: &T,
) -> Result<String, SubstError> {
    validate_subst(input)?;
    Ok(subst(map, input))
}

#[derive(Debug, Error)]
pub enum SubstError {
    #[error("Substitution Error for '{0}': The key '{1}' is malformed")]
    MalformedKey(String, String),
    #[error("Substitution Error: Input is empty")]
    EmptyInput,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_subst() {
        let mut map = HashMap::new();
        map.insert("key1".to_string(), "value1");
        map.insert("key2".to_string(), "value2");

        let input = "This is a ${key1} and this is a $key2 $key3";
        let expected_output = "This is a value1 and this is a value2 $key3";

        assert_eq!(subst(&map, &input), expected_output);
    }
    #[test]
    fn test_subst_validity() {
        let input = "This is a ${key1} and this is a $key2";
        let result = validate_subst(&input);
        assert!(result.is_ok());

        let input = "This is a ${} and this is a $key";
        let result = validate_subst(&input);
        assert!(result.is_err());

        let input = "This is a ${key and this is a $key";
        let result = validate_subst(&input);
        assert!(result.is_err());
    }
}
