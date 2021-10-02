use super::actionable_error::{ActionableError, ErrorCode};
use super::config::Config;
use super::script::Script;
use serde_json::Value;
use std::fs;

// This struct represents a project on the filesystem
pub struct Project {
    pub name: String,
    pub dir: String,
}

impl Project {
    // Try to create a project based on a name
    pub fn from_name(config: &Config, project_name: String) -> Result<Self, ActionableError> {
        let servers_dir = config.get_servers_dir();
        let package_json_path = format!("{}/{}/package.json", &servers_dir, project_name);
        let metadata = fs::metadata(format!(
            "{}/{}/package.json",
            servers_dir, project_name
        )).map_err(|_| {
            ActionableError {
                code: ErrorCode::ReadPackageJson,
                message: format!("Could not read {}", package_json_path),
                suggestion: format!("Try creating a new npm project in this project directory.\n\n    cd {}/{}\n    npm init", servers_dir, project_name),
            }
        })?;

        if !metadata.is_file() {
            return Err(ActionableError {
                code: ErrorCode::ReadPackageJson,
                message: format!("Could not read {}", package_json_path),
                suggestion: format!("Try making sure that {} is a file.", package_json_path),
            });
        }

        Ok(Project {
            name: project_name.clone(),
            dir: format!("{}/{}", servers_dir, project_name),
        })
    }

    // Return a vector of the project's start scripts
    pub fn get_start_scripts(&self) -> Result<Vec<Script>, ActionableError> {
        let package_json_path = format!("{}/package.json", self.dir);
        let package_json_content = fs::read_to_string(&package_json_path).map_err(|_| {
            ActionableError {
                code: ErrorCode::ReadPackageJson,
                message: format!("Could not read {}", package_json_path),
                suggestion: format!("Try creating a new npm project in this project directory.\n\n    cd {}\n    npm init", self.dir),
            }
        })?;
        let package_json: Value =
            serde_json::from_str(&package_json_content).map_err(|_| ActionableError {
                code: ErrorCode::ParsePackageJson,
                message: format!("Could not parse {}", package_json_path),
                suggestion: "Try making sure that package.json contains valid JSON.".to_string(),
            })?;
        let scripts = package_json["scripts"].as_object().and_then(|scripts| {
            if scripts.is_empty() { None } else { Some(scripts) }
        }).ok_or_else(|| ActionableError {
                code: ErrorCode::ParsePackageJson,
                message: format!("Property \"scripts\" in {} is not an object or is empty", package_json_path),
                suggestion: "Try making sure that the \"scripts\" property in package.json is an object with at least one key. For example:\n\n    \"scripts\": {\n        \"start\": \"node app.js\"\n    }".to_string(),
            })?;
        Ok(scripts
            .iter()
            .map(|(name, command)| Script {
                name: name.to_string(),
                command: command.to_string(),
            })
            .collect::<Vec<_>>())
    }

    // Determine whether the start script for a project is valid
    pub fn validate_start_script(&self, start_script: &str) -> Result<(), ActionableError> {
        let scripts = self.get_start_scripts()?;
        if !scripts.iter().any(|script| script.name == start_script) {
            let package_json_path = format!("{}/package.json", self.dir);
            return Err(ActionableError {
                code: ErrorCode::MissingStartScript,
                message: format!("No script {} in {}", start_script, package_json_path),
                suggestion: format!(
                    "Try adding the script {} to your package.json.",
                    start_script
                ),
            });
        }

        Ok(())
    }
}
