use std::collections::HashMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde::de::Error;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Yake {
    pub meta: YakeMeta,
    pub env: Vec<String>,
    pub targets: HashMap<String, YakeTarget>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct YakeMeta {
    pub doc: String,
    pub version: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct YakeTargetMeta {
    pub doc: String,
    #[serde(rename = "type")]
    pub target_type: YakeTargetType,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct YakeTarget {
    pub meta: YakeTargetMeta,
    pub targets: Option<HashMap<String, YakeTarget>>,
}

/// Custom deserialization via:
/// https://github.com/serde-rs/serde/issues/1019#issuecomment-322966402
#[derive(Debug, PartialEq)]
pub enum YakeTargetType {
    Group,
    Cmd,
}

impl Serialize for YakeTargetType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        serializer.serialize_str(match *self {
            YakeTargetType::Group => "group",
            YakeTargetType::Cmd => "cmd"
        })
    }
}

impl<'de> Deserialize<'de> for YakeTargetType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: Deserializer<'de>
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "group" => Ok(YakeTargetType::Group),
            "cmd" => Ok(YakeTargetType::Cmd),
            _ => Err(D::Error::custom(format!("unknown target type '{}'", s)))
        }
    }
}

impl Yake {
    pub fn get_targets(&self) -> Vec<String> {
        let mut ret = Vec::new();
        for (target_name, target) in &self.targets {
            if target.meta.target_type == YakeTargetType::Cmd {
                ret.push(target_name.clone());
            } else {
                let prefix = target_name.clone();
                ret.extend(target.get_targets(Some(prefix)).clone())
            }
        }

        ret
    }
}

impl YakeTarget {
    pub fn get_targets(&self, prefix: Option<String>) -> Vec<String> {
        let mut ret = Vec::new();
        match self.targets {
            Some(ref x) => for (target_name, target) in x {
                    if target.meta.target_type == YakeTargetType::Cmd {
                        let name = match prefix {
                            Some(ref x) => format!("{}.{}", x, target_name),
                            None => target_name.to_string(),
                        };
                        ret.push(name);
                    } else {
                        let p = match prefix {
                            Some(ref x) =>  Some(format!("{}.{}", x, target_name)),
                            None => None,
                        };
                        ret.extend(target.get_targets(p).clone())
                    }
                },
            None => ()

        }
        ret
    }
}