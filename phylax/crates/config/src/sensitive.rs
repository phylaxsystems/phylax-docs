use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{fmt::Debug, str::FromStr};

#[derive(Clone, Default, PartialEq)]
pub struct SensitiveValue<T>(T)
where
    T: for<'a> Deserialize<'a> + Serialize + Debug;

impl<'de, T> Deserialize<'de> for SensitiveValue<T>
where
    T: for<'a> Deserialize<'a> + Serialize + Debug + FromStr,
    T::Err: Debug,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = match T::deserialize(deserializer) {
            Ok(val) => val,
            Err(e) => return Err(e),
        };
        Ok(SensitiveValue(value))
    }
}

impl<T> Serialize for SensitiveValue<T>
where
    T: for<'a> Deserialize<'a> + Serialize + Debug,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str("<SENSITIVE>")
    }
}
impl<T> AsRef<T> for SensitiveValue<T>
where
    T: for<'a> Deserialize<'a> + Serialize + Debug,
{
    /// Returns a reference to the inner value
    fn as_ref(&self) -> &T {
        &self.0
    }
}
impl<T> AsMut<T> for SensitiveValue<T>
where
    T: for<'a> Deserialize<'a> + Serialize + Debug,
{
    /// Returns a mutable reference to the inner value
    fn as_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

impl<T> SensitiveValue<T>
where
    T: for<'a> Deserialize<'a> + Serialize + Debug,
{
    pub fn new(value: T) -> Self {
        Self(value)
    }
    /// Unwrap the inner value
    pub fn expose(&self) -> &T {
        &self.0
    }

    /// Consumes the SensitiveValue, returning the wrapped value
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> Debug for SensitiveValue<T>
where
    T: for<'a> Deserialize<'a> + Serialize + Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<SENSITIVE>")
    }
}

impl<T> std::fmt::Display for SensitiveValue<T>
where
    T: for<'a> Deserialize<'a> + Serialize + Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<SENSITIVE>")
    }
}

impl<T> FromStr for SensitiveValue<T>
where
    T: for<'a> Deserialize<'a> + Serialize + Debug + FromStr,
{
    type Err = <T as FromStr>::Err;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(SensitiveValue(s.parse()?))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{from_str, to_string};

    #[test]
    fn test_sensitive_value_serialize() {
        let sensitive = SensitiveValue::new("secret".to_string());
        let serialized = to_string(&sensitive).unwrap();
        assert_eq!(serialized, "\"<SENSITIVE>\"");
    }

    #[test]
    fn test_sensitive_value_deserialize() {
        let data = "\"secret\"";
        let deserialized: Result<SensitiveValue<String>, _> = from_str(data);
        assert!(deserialized.is_ok());
        assert_eq!(deserialized.unwrap().expose().to_string(), "secret");
    }

    #[test]
    fn test_sensitive_value_from_str() {
        let from_str_value: Result<SensitiveValue<String>, _> = "secret".parse();
        assert!(from_str_value.is_ok());
        assert_eq!(from_str_value.unwrap().expose().to_string(), "secret");
    }

    #[test]
    fn test_sensitive_value_as_ref() {
        let sensitive = SensitiveValue::new("test".to_string());
        let reference: &String = sensitive.as_ref();
        assert_eq!(reference, "test");
    }

    #[test]
    fn test_sensitive_value_as_mut() {
        let mut sensitive = SensitiveValue::new("test".to_string());
        let reference: &mut String = sensitive.as_mut();
        reference.clear();
        reference.push_str("changed");
        assert_eq!(sensitive.expose().to_string(), "changed");
    }

    #[test]
    fn test_sensitive_value_into_inner() {
        let sensitive = SensitiveValue::new("value".to_string());
        let inner = sensitive.into_inner();
        assert_eq!(inner, "value");
    }
}
