use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde::de::Error;
use std::collections::HashMap;
use std::process::{Command, Stdio};

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct Yake {
    pub meta: YakeMeta,
    pub env: Option<Vec<String>>,
    pub targets: HashMap<String, YakeTarget>,
    #[serde(skip)]
    fabricated: bool,
    #[serde(skip)]
    all_targets: HashMap<String, YakeTarget>,
    #[serde(skip)]
    dependencies: HashMap<String, Vec<YakeTarget>>,
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
    pub depends: Option<Vec<String>>,
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

    fn get_target_by_name(&self, target_name: &str) -> Option<YakeTarget> {
        self.get_all_targets().get(&target_name.to_string()).cloned()
    }

    fn get_all_dependencies(&self) -> HashMap<String, Vec<YakeTarget>> {
        let mut ret: HashMap<String, Vec<YakeTarget>> = HashMap::new();
        for (target_name, target) in self.get_all_targets() {
            ret.insert(target_name.clone(), Vec::new());
            for dependency_name in target.meta.depends.unwrap_or(vec![]).iter() {
                let dep = self.get_target_by_name(dependency_name);
                let dep_target = dep.expect(
                    format!("Warning: Unknown dependency: {} in target: {}.",
                            dependency_name,
                            target_name).as_str()
                );
                ret.get_mut(&target_name).unwrap().push(dep_target);
            }
        }

        ret
    }

    fn get_dependency_by_name(&self, target_name: &str) -> Vec<YakeTarget> {
        self.dependencies.get(target_name).unwrap().clone()
    }

    pub fn fabricate(&self) -> Yake {
        if self.fabricated {
            return self.clone();
        }

        let y = Yake {
            all_targets: self.get_all_targets(),
            dependencies: self.get_all_dependencies(),
            fabricated: true,
            ..self.clone()
        };

        return y;
    }

    pub fn execute(&self, target_name: &str) -> Result<String, String> {
        if self.has_target(target_name).is_err() {
            return Err(format!("Unknown target: {}", target_name).to_string());
        }

        let target = self.all_targets.get(target_name).unwrap();
        let dependencies = self.get_dependency_by_name(target_name);

        let run_target = |commands: Vec<String>| {
            for command in commands {
                println!("-- {}", command);
                Command::new("bash")
                    .arg("-c")
                    .arg(command.clone())
                    .stdout(Stdio::inherit())
                    .stderr(Stdio::inherit())
                    .output()
                    .expect(&format!("failed to execute command \"{}\"", command));
            }
        };

        for dep in dependencies {
            match dep.exec {
                Some(commands) => run_target(commands.to_vec()),
                _ => ()
            }
        }

        match target.exec {
            Some(ref commands) => run_target(commands.to_vec()),
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

#[allow(dead_code)]
fn get_all_targets<'a>(yake: &'a Yake) -> Vec<&'a YakeTarget> {
    let mut ret = Vec::new();

    ret.push(yake.targets.get("test").unwrap());

    ret
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ref() {
        let targets: HashMap<String, YakeTarget> =
            [("base".to_string(),
              YakeTarget {
                  targets: None,
                  meta: YakeTargetMeta {
                      doc: "Huhu".to_string(),
                      target_type: YakeTargetType::Cmd,
                      depends: None,
                  },
                  env: None,
                  exec: None,
              }),
                ("test".to_string(),
                 YakeTarget {
                     targets: None,
                     meta: YakeTargetMeta {
                         doc: "Huhu".to_string(),
                         target_type: YakeTargetType::Cmd,
                         depends: Some(vec!["base".to_string()]),
                     },
                     env: None,
                     exec: None,
                 })].iter().cloned().collect();

        let mut dependencies = HashMap::new();
        dependencies.insert("test".to_string(), vec![targets.get(&"base".to_string()).unwrap().clone()]);

        let yake = Yake {
            targets,
            dependencies,
            env: None,
            meta: YakeMeta {
                doc: "Bla".to_string(),
                version: "1.0.0".to_string(),
            },
            fabricated: false,
            all_targets: HashMap::new(),

        };

        let _targets = get_all_targets(&yake);
    }
}