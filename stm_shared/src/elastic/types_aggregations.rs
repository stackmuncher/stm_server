use juniper::{
    graphql_scalar,
    parser::{ParseError, ScalarToken, Token},
    serde::{de, Deserialize, Deserializer, Serialize},
    GraphQLObject, GraphQLScalarValue, ParseScalarResult, ScalarValue, Value,
};
use std::{convert::TryInto as _, fmt};

/// A generic structure for ES aggregations result. Make sure the aggregation name is `agg`.
/// ```json
///   {
///     "aggregations" : {
///       "agg" : {
///         "buckets" : [
///           {
///             "key" : "twilio",
///             "doc_count" : 597
///           }
///         ]
///       }
///     }
///   }
/// ```
#[derive(Deserialize, GraphQLObject, Serialize)]
#[graphql(scalar = MyScalarValue)]
pub struct ESAggs {
    pub aggregations: ESAggsAgg,
}

/// Part of ESAggs
#[derive(Deserialize, GraphQLObject, Serialize)]
#[graphql(scalar = MyScalarValue)]
pub struct ESAggsBucket {
    pub key: String,
    pub doc_count: u64,
}

/// Part of ESAggs
#[derive(Deserialize, GraphQLObject, Serialize)]
#[graphql(scalar = MyScalarValue)]
pub struct ESAggsBuckets {
    pub buckets: Vec<ESAggsBucket>,
}

/// Part of ESAggs
#[derive(Deserialize, GraphQLObject, Serialize)]
#[graphql(scalar = MyScalarValue)]
pub struct ESAggsAgg {
    pub agg: ESAggsBuckets,
}

impl Default for ESAggs {
    fn default() -> Self {
        serde_json::from_str(r#"{"aggregations" : {"agg" : {"buckets" : [{"key" : "twilio","doc_count" : 597}]}}}"#)
            .unwrap()
    }
}

// ----- JUNIPER -----

#[graphql_scalar(name = "Long")]
impl GraphQLScalar for u64 {
    fn resolve(&self) -> Value {
        Value::scalar(*self)
    }

    fn from_input_value(v: &InputValue) -> Result<u64, String> {
        v.as_scalar_value::<u64>()
            .copied()
            .ok_or_else(|| format!("Expected `MyScalarValue::Long`, found: {}", v))
    }

    fn from_str<'a>(value: ScalarToken<'a>) -> ParseScalarResult<'a, MyScalarValue> {
        if let ScalarToken::Int(v) = value {
            v.parse()
                .map_err(|_| ParseError::UnexpectedToken(Token::Scalar(value)))
                .map(|s: u64| s.into())
        } else {
            Err(ParseError::UnexpectedToken(Token::Scalar(value)))
        }
    }
}

#[derive(GraphQLScalarValue, Clone, Debug, PartialEq, Serialize)]
#[serde(untagged)]
pub enum MyScalarValue {
    Int(i32),
    Long(u64),
    Float(f64),
    String(String),
    Boolean(bool),
}

impl ScalarValue for MyScalarValue {
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

impl<'de> Deserialize<'de> for MyScalarValue {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = MyScalarValue;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a valid input value")
            }

            fn visit_bool<E: de::Error>(self, b: bool) -> Result<Self::Value, E> {
                Ok(MyScalarValue::Boolean(b))
            }

            fn visit_i32<E: de::Error>(self, n: i32) -> Result<Self::Value, E> {
                Ok(MyScalarValue::Int(n))
            }

            fn visit_u64<E: de::Error>(self, b: u64) -> Result<Self::Value, E> {
                if b <= u64::from(u32::MAX) {
                    self.visit_i32(b.try_into().unwrap())
                } else {
                    Ok(MyScalarValue::Long(b))
                }
            }

            fn visit_u32<E: de::Error>(self, n: u32) -> Result<Self::Value, E> {
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
                    Ok(MyScalarValue::Float(n as f64))
                }
            }

            fn visit_f64<E: de::Error>(self, f: f64) -> Result<Self::Value, E> {
                Ok(MyScalarValue::Float(f))
            }

            fn visit_str<E: de::Error>(self, s: &str) -> Result<Self::Value, E> {
                self.visit_string(s.into())
            }

            fn visit_string<E: de::Error>(self, s: String) -> Result<Self::Value, E> {
                Ok(MyScalarValue::String(s))
            }
        }

        de.deserialize_any(Visitor)
    }
}
