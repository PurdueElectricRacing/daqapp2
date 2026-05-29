use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;

pub const FORMATTER_CONFIG_FILE: &str = "formatter_config.json";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Formatting {
    Hex,
    Binary,
    Decimal(i32), // Number of decimal places
}

// Message name/pattern -> signal name/pattern -> formatting
type FormatterConfig = HashMap<String, HashMap<String, Formatting>>;
type CompiledFormatterConfig = Vec<(
    globset::GlobMatcher,
    Vec<(globset::GlobMatcher, Formatting)>,
)>;

impl Serialize for Formatting {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Formatting::Hex => serializer.serialize_str("hex"),
            Formatting::Binary => serializer.serialize_str("binary"),
            Formatting::Decimal(places) => serializer.serialize_i32(*places),
        }
    }
}

impl<'de> Deserialize<'de> for Formatting {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct FormattingVisitor;

        impl<'de> serde::de::Visitor<'de> for FormattingVisitor {
            type Value = Formatting;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str(r#"an integer, "hex", or "binary""#)
            }

            fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                let value =
                    i32::try_from(value).map_err(|_| E::custom("decimal places out of range"))?;

                Ok(Formatting::Decimal(value))
            }

            fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                let value =
                    i32::try_from(value).map_err(|_| E::custom("decimal places out of range"))?;

                Ok(Formatting::Decimal(value))
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match value {
                    "hex" => Ok(Formatting::Hex),
                    "binary" => Ok(Formatting::Binary),
                    _ => Err(E::unknown_variant(value, &["hex", "binary"])),
                }
            }
        }

        deserializer.deserialize_any(FormattingVisitor)
    }
}

