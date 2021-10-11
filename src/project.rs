use super::config::Config;
use super::error::ApplicationError;
use super::script::Script;
use serde_json::Value;
use std::fmt;
use std::fs;
use std::path::PathBuf;

// This struct represents a project on the filesystem
#[derive(Clone, Debug)]
pub struct Project {
    pub name: String,
    pub dir: PathBuf,
}

impl fmt::Display for Project {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> fmt::Result {
        write!(formatter, "{}", self.name)
    }
}

impl Project {
    // Try to create a project based on a path
    pub fn from_path(project_path: PathBuf) -> Result<Self, ApplicationError> {
        let name = project_path
            .file_name()
            .ok_or_else(|| ApplicationError::ParsePath(project_path.clone()))?
            .to_str()
            .ok_or_else(|| ApplicationError::ParsePath(project_path.clone()))?
            .to_string();
        let project = Project {
            name,
            dir: project_path,
        };
        let package_json_path = project.get_package_json();
        let metadata = fs::metadata(&package_json_path)
            .map_err(|_| ApplicationError::ReadPackageJson(package_json_path.clone()))?;

        if !metadata.is_file() {
            return Err(ApplicationError::ReadPackageJson(package_json_path));
        }

        Ok(project)
    }

    // Try to create a project based on a name
    pub fn from_name(config: &Config, project_name: String) -> Result<Self, ApplicationError> {
        Project::from_path(config.get_servers_dir().join(project_name))
    }

    // Return a vector of the project's start scripts
    pub fn get_start_scripts(&self) -> Result<Vec<Script>, ApplicationError> {
        let package_json_path = self.get_package_json();
        let package_json_content = fs::read_to_string(&package_json_path)
            .map_err(|_| ApplicationError::ReadPackageJson(package_json_path.clone()))?;
        let package_json: Value = serde_json::from_str(&package_json_content).map_err(|_| {
            ApplicationError::MalformedPackageJson {
                path: package_json_path.clone(),
                cause: "contains invalid JSON".to_string(),
            }
        })?;
        let scripts = package_json["scripts"].as_object().ok_or_else(|| {
            ApplicationError::MalformedPackageJson {
                path: package_json_path.clone(),
                cause: "\"scripts\" property is not an object".to_string(),
            }
        })?;
        if scripts.is_empty() {
            return Err(ApplicationError::MalformedPackageJson {
                path: package_json_path,
                cause: "\"scripts\" is an empty object".to_string(),
            });
        }
        Ok(scripts
            .iter()
            .map(|(name, command)| Script {
                name: name.to_string(),
                command: command.to_string(),
            })
            .collect::<Vec<_>>())
    }

    // Determine whether the start script for a project is valid
    pub fn validate_start_script(&self, start_script: &str) -> Result<(), ApplicationError> {
        let scripts = self.get_start_scripts()?;
        if !scripts.iter().any(|script| script.name == start_script) {
            return Err(ApplicationError::NonExistentScript {
                project: self.clone(),
                script: start_script.to_string(),
            });
        }

        Ok(())
    }

    // Return the path to the project's package.json file
    pub fn get_package_json(&self) -> PathBuf {
        self.dir.join("package.json")
    }
}
