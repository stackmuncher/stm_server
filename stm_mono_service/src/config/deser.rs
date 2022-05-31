use serde::de::{Deserializer, Error, Visitor};
use serde::Deserialize;
use std::fmt;
use std::ops::Deref;
use std::str::FromStr;

// ***** AWS REGION DESERIALIZATION *****

type AwsRegion = rusoto_core::region::Region;

struct RegionVisitor;
impl<'de> Visitor<'de> for RegionVisitor {
    type Value = super::Region;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("Must be a valid region string, e.g. us-east-1")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        match AwsRegion::from_str(v) {
            Ok(rng) => Ok(super::Region(rng)),
            Err(e) => Err(E::custom(e)),
        }
    }
}

impl<'de> Deserialize<'de> for super::Region {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(RegionVisitor)
    }
}

impl Deref for super::Region {
    type Target = AwsRegion;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// ***** CONFIG::FLOW DESERIALIZATION *****

struct FlowVisitor;
impl<'de> Visitor<'de> for FlowVisitor {
    type Value = super::Flow;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str(&format!("Expecting {:?}", super::Config::CLI_MODES))
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        match super::Flow::from_str(v) {
            Ok(flow) => Ok(flow),
            Err(_) => unreachable!(),
        }
    }
}

impl<'de> Deserialize<'de> for super::Flow {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(FlowVisitor)
    }
}

// ***** LOG::LEVEL DESERIALIZATION *****

impl Default for super::LogLevel {
    /// Returns Flow::Help
    fn default() -> Self {
        super::LogLevel(tracing::Level::INFO)
    }
}

impl Deref for super::LogLevel {
    type Target = tracing::Level;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

struct LogLevelVisitor;
impl<'de> Visitor<'de> for LogLevelVisitor {
    type Value = super::LogLevel;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str(&format!("Expecting a valid log level, e.g. info, debug, etc"))
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        Ok(super::LogLevel(super::Config::string_to_log_level(v)))
    }
}

impl<'de> Deserialize<'de> for super::LogLevel {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(LogLevelVisitor)
    }
}
