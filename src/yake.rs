use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde::de::Error;
use std::collections::HashMap;
use std::process::{Command, Stdio};

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct Yake {
    pub meta: YakeMeta,
    pub env: Vec<String>,
    pub targets: HashMap<String, YakeTarget>,
    #[serde(skip)]
    flattened: bool,
    #[serde(skip)]
    all_targets: HashMap<String, YakeTarget>,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct YakeMeta {
    pub doc: String,
    pub version: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct YakeTargetMeta {
    pub doc: String,
    #[serde(rename = "type")]
    pub target_type: YakeTargetType,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct YakeTarget {
    pub meta: YakeTargetMeta,
    pub targets: Option<HashMap<String, YakeTarget>>,
    pub env: Option<Vec<String>>,
    pub exec: Option<Vec<String>>,
}

/// Custom deserialization via:
/// https://github.com/serde-rs/serde/issues/1019#issuecomment-322966402
#[derive(Debug, PartialEq, Clone)]
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
        for (target_name, target) in &self.all_targets {
            if target.meta.target_type == YakeTargetType::Cmd {
                ret.push(target_name.clone());
            }
        }

        ret
    }

    fn get_all_targets(&self) -> HashMap<String, YakeTarget> {
        let mut targets = HashMap::new();

        for (target_name, target) in &self.targets {
            targets.insert(target_name.to_string(), target.clone());
            if target.targets.is_some() {
                let prefix = target_name.clone();
                targets.extend(target.get_sub_targets(Some(prefix)));
            }
        }

        targets
    }

    pub fn has_target(&self, target_name: &str) -> Result<(), Vec<String>> {
        if self.get_targets().contains(&target_name.to_string()) {
            Ok(())
        } else {
            Err(self.get_targets().clone())
        }
    }

    pub fn flatten(&self) -> Yake {
        if self.flattened {
            return self.clone();
        }

        let mut y = self.clone();
        y.all_targets = self.get_all_targets().clone();
        y.flattened = true;
        return y;
    }

    pub fn execute(&self, target_name: &str) -> Result<String, String> {
        if self.has_target(target_name).is_err() {
            return Err(format!("Unknown target: {}", target_name).to_string());
        }

        let target = self.all_targets.get(target_name).unwrap();
        match target.exec {
            Some(ref x) => {
                for cmd in x {
                    println!("-- {}", cmd);
                    Command::new("bash")
                        .arg("-c")
                        .arg(cmd)
                        .stdout(Stdio::inherit())
                        .stderr(Stdio::inherit())
                        .output()
                        .expect(&format!("failed to execute command \"{}\"", cmd));
                }
            }
            _ => ()
        }

        Ok("All cool".to_string())
    }
}

impl YakeTarget {
    pub fn get_sub_targets(&self, prefix: Option<String>) -> HashMap<String, YakeTarget> {
        let mut targets = HashMap::new();
        match self.targets {
            Some(ref x) => for (target_name, target) in x {
                if target.meta.target_type == YakeTargetType::Cmd {
                    let name = match prefix {
                        Some(ref x) => format!("{}.{}", x, target_name),
                        None => target_name.to_string(),
                    };
                    targets.insert(name, target.clone());
                } else {
                    let p = match prefix {
                        Some(ref x) => Some(format!("{}.{}", x, target_name)),
                        None => None,
                    };
                    targets.extend(target.get_sub_targets(p))
                }
            },
            None => ()
        }
        targets
    }
}