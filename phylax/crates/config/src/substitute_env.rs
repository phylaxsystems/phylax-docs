use std::{
    collections::HashMap,
    env,
    fmt::{self, Debug},
    str::FromStr,
};

use serde::{
    de::{self, SeqAccess},
    Deserialize, Deserializer,
};

pub fn deserialize_with_env<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Debug + FromStr,
    T::Err: Debug,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    let substituted = phylax_common::subst_and_validate(&get_env_vars(), &s)
        .map_err(|err| de::Error::custom(format!("{:?}", err)))?;
    substituted.parse::<T>().map_err(|err| de::Error::custom(format!("{:?}", err)))
}

/// Deserialize a sequence with environment variables substitution
pub fn deserialize_seq_with_env<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Debug + FromStr,
    T::Err: Debug,
{
    struct SeqVisitor<T> {
        marker: std::marker::PhantomData<T>,
    }

    impl<'de, T> de::Visitor<'de> for SeqVisitor<T>
    where
        T: Debug + FromStr,
        T::Err: Debug,
    {
        type Value = Vec<T>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a sequence")
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: SeqAccess<'de>,
        {
            let mut values = Vec::new();
            while let Some(value) = seq.next_element::<String>()? {
                let substituted = phylax_common::subst_and_validate(&get_env_vars(), &value)
                    .map_err(|err| de::Error::custom(format!("{:?}", err)))?;
                let parsed = substituted
                    .parse::<T>()
                    .map_err(|err| de::Error::custom(format!("{:?}", err)))?;
                values.push(parsed);
            }
            Ok(values)
        }
    }

    deserializer.deserialize_seq(SeqVisitor { marker: std::marker::PhantomData })
}

/// Get all the environment variables and return them
pub fn get_env_vars() -> HashMap<String, String> {
    let env_vars = env::vars();
    env_vars.collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_get_env_vars() {
        // Set an environment variable
        env::set_var("PH_TEST", "test_value");
        let env_vars = get_env_vars();
        assert_eq!(env_vars.get("PH_TEST"), Some(&"test_value".to_string()));
    }
}
