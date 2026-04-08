//! Wing - person or project container

use serde::{Deserialize, Serialize};

/// Type of wing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum WingType {
    Person,
    #[default]
    Project,
}

/// Wing definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wing {
    pub name: String,
    pub wing_type: WingType,
    pub keywords: Vec<String>,
}

impl Wing {
    pub fn new(name: impl Into<String>, wing_type: WingType, keywords: Vec<String>) -> Self {
        Self {
            name: name.into(),
            wing_type,
            keywords,
        }
    }

    pub fn person(name: impl Into<String>, keywords: Vec<String>) -> Self {
        Self {
            name: name.into(),
            wing_type: WingType::Person,
            keywords,
        }
    }

    pub fn project(name: impl Into<String>, keywords: Vec<String>) -> Self {
        Self {
            name: name.into(),
            wing_type: WingType::Project,
            keywords,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wing_person() {
        let wing = Wing::person("kai", vec!["kai".into(), "daughter".into()]);
        assert_eq!(wing.name, "kai");
        assert_eq!(wing.wing_type, WingType::Person);
        assert!(wing.keywords.contains(&"kai".into()));
    }

    #[test]
    fn test_wing_project() {
        let wing = Wing::project("mempalace", vec!["rust".into(), "memory".into()]);
        assert_eq!(wing.name, "mempalace");
        assert_eq!(wing.wing_type, WingType::Project);
    }
}
