use juniper::{
    graphql_scalar,
    parser::{ParseError, ScalarToken, Token},
    serde::{de, Deserialize, Deserializer, Serialize},
    GraphQLScalarValue, ParseScalarResult, ScalarValue, Value,
};
use std::{convert::TryInto as _, fmt};

/// An extension to the standard GraphQL set of types to include Rust scalar values.
/// Only the types used in this project are added to the list.
/// ### About GraphQL scalars
/// * https://graphql.org/learn/schema/#scalar-types
/// * https://www.graphql-tools.com/docs/scalars#custom-scalars
/// ### About extending the GraphQL scalars in Juniper
/// * https://graphql-rust.github.io/juniper/master/types/scalars.html#custom-scalars
/// * https://github.com/graphql-rust/juniper/issues/862
///
#[derive(GraphQLScalarValue, Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum RustScalarValue {
    /// A GraphQL scalar for i32
    Int(i32),
    /// A custom scalar for u64. The value is serialized into JSON number and should not be more than 53 bits to fit into JS Number type:
    /// * Number.MAX_SAFE_INTEGER = 2^53 - 1 = 9_007_199_254_740_991
    /// * https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Number
    /// JSON spec does not constrain integer values unless specified in the schema. 53 bits is sufficient for our purposes.
    U64(u64),
    /// A GraphQL scalar for f64
    Float(f64),
    /// A GraphQL scalar for String
    String(String),
    /// A GraphQL scalar for bool
    Boolean(bool),
}

#[graphql_scalar(name = "U64")]
impl GraphQLScalar for u64 {
    fn resolve(&self) -> Value {
        Value::scalar(*self)
    }

    fn from_input_value(v: &InputValue) -> Result<u64, String> {
        v.as_scalar_value::<u64>()
            .copied()
            .ok_or_else(|| format!("Expected `RustScalarValue::U64`, found: {}", v))
    }

    fn from_str<'a>(value: ScalarToken<'a>) -> ParseScalarResult<'a, RustScalarValue> {
        if let ScalarToken::Int(v) = value {
            v.parse()
                .map_err(|_| ParseError::UnexpectedToken(Token::Scalar(value)))
                .map(|s: u64| s.into())
        } else {
            Err(ParseError::UnexpectedToken(Token::Scalar(value)))
        }
    }
}

impl ScalarValue for RustScalarValue {
    fn as_int(&self) -> Option<i32> {
        match self {
            Self::Int(i) => Some(*i),
            _ => None,
        }
    }

    fn as_string(&self) -> Option<String> {
        match self {
            Self::String(s) => Some(s.clone()),
            _ => None,
        }
    }

    fn into_string(self) -> Option<String> {
        match self {
            Self::String(s) => Some(s),
            _ => None,
        }
    }

    fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s.as_str()),
            _ => None,
        }
    }

    fn as_float(&self) -> Option<f64> {
        match self {
            Self::Int(i) => Some(f64::from(*i)),
            Self::Float(f) => Some(*f),
            _ => None,
        }
    }

    fn as_boolean(&self) -> Option<bool> {
        match self {
            Self::Boolean(b) => Some(*b),
            _ => None,
        }
    }
}

impl<'de> Deserialize<'de> for RustScalarValue {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = RustScalarValue;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a valid input value")
            }

            fn visit_bool<E: de::Error>(self, b: bool) -> Result<Self::Value, E> {
                Ok(RustScalarValue::Boolean(b))
            }

            fn visit_i32<E: de::Error>(self, n: i32) -> Result<Self::Value, E> {
                Ok(RustScalarValue::Int(n))
            }

            fn visit_u64<E: de::Error>(self, b: u64) -> Result<Self::Value, E> {
                // I do not understand why this IF is needed
                if b <= u64::from(u32::MAX) {
                    self.visit_i32(b.try_into().unwrap())
                } else {
                    Ok(RustScalarValue::U64(b))
                }
            }

            fn visit_u32<E: de::Error>(self, n: u32) -> Result<Self::Value, E> {
                // I do not understand why this IF is needed
                if n <= i32::MAX as u32 {
                    self.visit_i32(n.try_into().unwrap())
                } else {
                    self.visit_u64(n.into())
                }
            }

            fn visit_i64<E: de::Error>(self, n: i64) -> Result<Self::Value, E> {
                if n <= i64::MAX as i64 {
                    self.visit_i64(n.try_into().unwrap())
                } else {
                    // Browser's `JSON.stringify()` serializes all numbers
                    // having no fractional part as integers (no decimal point),
                    // so we must parse large integers as floating point,
                    // otherwise we would error on transferring large floating
                    // point numbers.
                    // TODO: Use `FloatToInt` conversion once stabilized:
                    //       https://github.com/rust-lang/rust/issues/67057
                    Ok(RustScalarValue::Float(n as f64))
                }
            }

            fn visit_f64<E: de::Error>(self, f: f64) -> Result<Self::Value, E> {
                Ok(RustScalarValue::Float(f))
            }

            fn visit_str<E: de::Error>(self, s: &str) -> Result<Self::Value, E> {
                self.visit_string(s.into())
            }

            fn visit_string<E: de::Error>(self, s: String) -> Result<Self::Value, E> {
                Ok(RustScalarValue::String(s))
            }
        }

        de.deserialize_any(Visitor)
    }
}
