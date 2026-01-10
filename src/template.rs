use anyhow::{Context, Result};
use handlebars::Handlebars;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub struct TemplateEngine {
    handlebars: Handlebars<'static>,
}

impl TemplateEngine {
    pub fn new() -> Self {
        let mut handlebars = Handlebars::new();
        handlebars.set_strict_mode(false); // Allow missing vars with default syntax
        handlebars.register_escape_fn(handlebars::no_escape); // Don't escape output

        Self { handlebars }
    }

    pub fn render_file(
        &self,
        source: &Path,
        vars: &HashMap<String, String>,
    ) -> Result<String> {
        let content = fs::read_to_string(source)
            .with_context(|| format!("Failed to read template: {}", source.display()))?;

        self.render_string(&content, vars)
    }

    pub fn render_string(
        &self,
        template: &str,
        vars: &HashMap<String, String>,
    ) -> Result<String> {
        // Pre-process template for our custom syntax:
        // {{var?}} -> optional (empty if missing)
        // {{var:default}} -> default value if missing
        let processed = preprocess_template(template, vars);

        self.handlebars
            .render_template(&processed, vars)
            .context("Failed to render template")
    }
}

impl Default for TemplateEngine {
    fn default() -> Self {
        Self::new()
    }
}

fn preprocess_template(template: &str, vars: &HashMap<String, String>) -> String {
    let mut result = template.to_string();

    // Handle env vars first ({{env.VAR_NAME}}) - replace directly since Handlebars
    // doesn't support dots in variable names
    let env_re = regex_lite::Regex::new(r"\{\{(env\.\w+)\}\}").unwrap();
    result = env_re
        .replace_all(&result, |caps: &regex_lite::Captures| {
            let var_name = &caps[1];
            vars.get(var_name).cloned().unwrap_or_default()
        })
        .to_string();

    // Handle {{var:default}} syntax
    let default_re = regex_lite::Regex::new(r"\{\{(\w+):([^}]*)\}\}").unwrap();
    result = default_re
        .replace_all(&result, |caps: &regex_lite::Captures| {
            let var_name = &caps[1];
            let default = &caps[2];
            if vars.contains_key(var_name) {
                format!("{{{{{}}}}}", var_name)
            } else {
                default.to_string()
            }
        })
        .to_string();

    // Handle {{var?}} syntax (optional, empty if missing)
    let optional_re = regex_lite::Regex::new(r"\{\{(\w+)\?\}\}").unwrap();
    result = optional_re
        .replace_all(&result, |caps: &regex_lite::Captures| {
            let var_name = &caps[1];
            if vars.contains_key(var_name) {
                format!("{{{{{}}}}}", var_name)
            } else {
                String::new()
            }
        })
        .to_string();

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_render() {
        let engine = TemplateEngine::new();
        let mut vars = HashMap::new();
        vars.insert("name".to_string(), "World".to_string());

        let result = engine.render_string("Hello, {{name}}!", &vars).unwrap();
        assert_eq!(result, "Hello, World!");
    }

    #[test]
    fn test_default_value() {
        let engine = TemplateEngine::new();
        let vars = HashMap::new();

        let result = engine
            .render_string("Hello, {{name:Guest}}!", &vars)
            .unwrap();
        assert_eq!(result, "Hello, Guest!");
    }

    #[test]
    fn test_default_value_with_var_present() {
        let engine = TemplateEngine::new();
        let mut vars = HashMap::new();
        vars.insert("name".to_string(), "World".to_string());

        let result = engine
            .render_string("Hello, {{name:Guest}}!", &vars)
            .unwrap();
        assert_eq!(result, "Hello, World!");
    }

    #[test]
    fn test_optional_missing() {
        let engine = TemplateEngine::new();
        let vars = HashMap::new();

        let result = engine.render_string("Hello{{name?}}!", &vars).unwrap();
        assert_eq!(result, "Hello!");
    }

    #[test]
    fn test_optional_present() {
        let engine = TemplateEngine::new();
        let mut vars = HashMap::new();
        vars.insert("name".to_string(), ", World".to_string());

        let result = engine.render_string("Hello{{name?}}!", &vars).unwrap();
        assert_eq!(result, "Hello, World!");
    }

    #[test]
    fn test_env_var_syntax() {
        let engine = TemplateEngine::new();
        let mut vars = HashMap::new();
        vars.insert("env.API_KEY".to_string(), "secret123".to_string());

        let result = engine
            .render_string("Key: {{env.API_KEY}}", &vars)
            .unwrap();
        assert_eq!(result, "Key: secret123");
    }

    #[test]
    fn test_multiline_template() {
        let engine = TemplateEngine::new();
        let mut vars = HashMap::new();
        vars.insert("user".to_string(), "alice".to_string());
        vars.insert("email".to_string(), "alice@example.com".to_string());

        let template = r#"[user]
    name = {{user}}
    email = {{email}}
"#;

        let expected = r#"[user]
    name = alice
    email = alice@example.com
"#;

        let result = engine.render_string(template, &vars).unwrap();
        assert_eq!(result, expected);
    }
}
