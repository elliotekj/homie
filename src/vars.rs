use std::collections::HashMap;
use std::env;

use crate::config::GlobalConfig;
use crate::repo::Repo;

pub struct VarResolver {
    vars: HashMap<String, String>,
    env_passthrough: Vec<String>,
}

impl VarResolver {
    pub fn new(global_config: &GlobalConfig, repo: Option<&Repo>) -> Self {
        let mut vars = HashMap::new();

        // Built-in vars
        vars.insert("hostname".to_string(), get_hostname());
        vars.insert("user".to_string(), get_username());
        vars.insert("home".to_string(), get_home_dir());
        vars.insert("os".to_string(), get_os());

        // Global vars
        for (k, v) in &global_config.vars {
            vars.insert(k.clone(), v.clone());
        }

        // Repo-specific vars (override globals)
        if let Some(r) = repo {
            for (k, v) in r.vars() {
                vars.insert(k.clone(), v.clone());
            }
        }

        Self {
            vars,
            env_passthrough: global_config.env.pass_through.clone(),
        }
    }

    pub fn to_template_data(&self) -> HashMap<String, String> {
        let mut data = self.vars.clone();

        // Add passthrough env vars
        for env_var in &self.env_passthrough {
            if let Ok(value) = env::var(env_var) {
                data.insert(format!("env.{}", env_var), value);
            }
        }

        data
    }
}

fn get_hostname() -> String {
    hostname::get()
        .ok()
        .and_then(|h| h.into_string().ok())
        .unwrap_or_else(|| "unknown".to_string())
}

fn get_username() -> String {
    env::var("USER")
        .or_else(|_| env::var("USERNAME"))
        .unwrap_or_else(|_| "unknown".to_string())
}

fn get_home_dir() -> String {
    dirs::home_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "~".to_string())
}

fn get_os() -> String {
    #[cfg(target_os = "macos")]
    return "macos".to_string();
    #[cfg(target_os = "linux")]
    return "linux".to_string();
    #[cfg(target_os = "windows")]
    return "windows".to_string();
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    return "unknown".to_string();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_vars() {
        let config = GlobalConfig::default();
        let resolver = VarResolver::new(&config, None);

        let data = resolver.to_template_data();
        assert!(data.contains_key("hostname"));
        assert!(data.contains_key("user"));
        assert!(data.contains_key("home"));
        assert!(data.contains_key("os"));
    }

    #[test]
    fn test_global_vars() {
        let mut config = GlobalConfig::default();
        config.vars.insert("email".to_string(), "test@example.com".to_string());

        let resolver = VarResolver::new(&config, None);
        let data = resolver.to_template_data();
        assert_eq!(data.get("email"), Some(&"test@example.com".to_string()));
    }
}
