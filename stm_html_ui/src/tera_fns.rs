use serde_json::{from_value, to_value, Number, Value};
use std::collections::HashMap;
use tera::Function;

/// Shortens long numbers to abbreviations, e.g. 1000 to 1K, 25343 to 25K.
/// ## Example
/// ```no-run
/// shorten_num(v=tech.loc)
/// ```
pub(crate) fn shorten_num() -> impl Function {
    Box::new(move |args: &HashMap<String, Value>| -> Result<Value, tera::Error> {
        match args.get("v") {
            Some(val) => match from_value::<Number>(val.clone()) {
                Ok(v) => match v.as_u64() {
                    Some(v) => {
                        // add commas
                        let txt = if v < 100 {
                            "<100".to_string()
                        } else if v >= 100 && v < 1000 {
                            v.to_string()
                        } else if v >= 1_000 && v < 10_000 {
                            format!("{:.1}K", v as f64 / 1000.0)
                        } else if v >= 10_000 && v < 1_000_000 {
                            format!("{}K", v / 1000)
                        } else {
                            format!("{:.1}M", v as f64 / 1_000_000.0)
                        };

                        match to_value(txt) {
                            Ok(val) => Ok(val),
                            Err(_) => Ok(val.clone()),
                        }
                    }
                    None => Ok(val.clone()),
                },
                Err(_) => Ok(val.clone()),
            },
            None => Ok(Value::Null),
        }
    })
}

/// Inserts commas into integers for readability, e.g. 1000 -> 1,000
/// ## Example
/// ```no-run
/// pretty_num(v=tech.loc)
/// ```
pub(crate) fn pretty_num() -> impl Function {
    Box::new(move |args: &HashMap<String, Value>| -> Result<Value, tera::Error> {
        match args.get("v") {
            Some(val) => match from_value::<Number>(val.clone()) {
                Ok(v) => match v.as_f64() {
                    Some(v) => {
                        // discard - and any factions
                        let v = (v.round() as i64).unsigned_abs().to_string();

                        // a container for a ascii chars with commas
                        let mut txt: Vec<u8> = Vec::new();
                        let v_as_bytes = v.as_bytes();
                        txt.reserve_exact(v_as_bytes.len() + v_as_bytes.len() / 3);

                        for (i, c) in v_as_bytes.into_iter().rev().into_iter().enumerate() {
                            // insert a comma after every 3rd digit
                            if i > 0 && i < v_as_bytes.len() && i % 3 == 0 {
                                txt.push(44);
                            }
                            txt.push(*c);
                        }

                        // convert it back into a legit UTF8 string
                        txt.reverse();
                        let txt = String::from_utf8_lossy(txt.as_slice()).to_string();

                        // return Value::<String>
                        match to_value(txt) {
                            Ok(val) => Ok(val),
                            Err(_) => Ok(val.clone()),
                        }
                    }
                    None => Ok(val.clone()),
                },
                Err(_) => Ok(val.clone()),
            },
            None => Ok(Value::Null),
        }
    })
}
