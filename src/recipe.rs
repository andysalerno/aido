//! Recipe parsing and management for the aido tool
//!
//! This module handles the parsing and management of recipe files, which contain
//! YAML frontmatter headers and markdown body content. Recipes define templates
//! for AI interactions with specific tools and configurations.

use std::path::Path;
use std::sync::LazyLock;

use log::info;
use regex::Regex;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Custom error types for recipe operations
#[derive(Error, Debug)]
pub enum RecipeError {
    #[error("Recipe '{name}' not found")]
    NotFound { name: String },

    #[error("Recipe content is empty")]
    EmptyContent,

    #[error("Invalid recipe format: {message}")]
    InvalidFormat { message: String },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("YAML parsing error: {0}")]
    Yaml(#[from] serde_yaml::Error),
}

/// Regex pattern to match YAML frontmatter delimiters in recipe files
///
/// The pattern captures:
/// 1. Opening delimiter (3 or more dashes) at the very beginning of the document
/// 2. Header content (YAML frontmatter, using non-greedy match including newlines)
/// 3. Closing delimiter (3 or more dashes) on its own line
/// 4. Remaining body content (everything after the closing delimiter)
///
/// Example matches:
/// ```text
/// ---
/// name: example
/// allowed_tools: [ls, cat]
/// ---
/// This is the body content.
/// ```
static HEADER_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)^(-{3,})\s*\n(.*?)\n(-{3,})\s*\n(.*)$").unwrap()
});

/// A recipe file containing a YAML header and markdown body
#[derive(Debug, Clone)]
pub struct Recipe {
    /// The YAML frontmatter header of the recipe
    header: Header,
    /// The body content of the recipe
    body: String,
}

impl Recipe {
    /// Create a new recipe with the given header and body
    pub fn new(header: Header, body: String) -> Self {
        Self { header, body }
    }

    /// Get a reference to the recipe's header
    #[must_use]
    pub fn header(&self) -> &Header {
        &self.header
    }

    /// Get the recipe's body content
    #[must_use]
    pub fn body(&self) -> &str {
        &self.body
    }
}

/// Header information parsed from the YAML frontmatter
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Header {
    /// The name of the recipe
    #[serde(default)]
    name: String,
    /// List of tools allowed to be used by this recipe
    #[serde(default)]
    allowed_tools: Vec<String>,
}

impl Header {
    /// Parse header content from YAML string
    fn parse(content: &str) -> Result<Self, RecipeError> {
        if content.trim().is_empty() {
            return Ok(Self::default());
        }

        // Try to parse as YAML first, but fall back to empty header if it fails
        serde_yaml::from_str(content).or_else(|_| {
            // If YAML parsing fails, return an empty header
            // This maintains backwards compatibility with non-YAML headers
            Ok(Self::default())
        })
    }

    /// Create an empty header
    fn empty() -> Self {
        Self::default()
    }

    /// Get the recipe name
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the list of allowed tools
    #[must_use]
    pub fn allowed_tools(&self) -> &[String] {
        &self.allowed_tools
    }
}

/// Information about a recipe file
#[derive(Debug, Clone)]
pub struct RecipeInfo {
    /// The name of the recipe (filename without extension)
    pub name: String,
    /// The display name from the recipe header
    pub display_name: String,
}

/// Lists all available recipes in the recipes directory
pub fn list(config_file_path: &str) -> Result<Vec<RecipeInfo>, RecipeError> {
    let recipe_dir = get_recipes_dir(config_file_path);
    let mut recipes = Vec::new();

    let entries = std::fs::read_dir(&recipe_dir)?;

    for entry in entries
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|ft| ft.is_file()))
    {
        if let Some(filename) = entry.file_name().to_str() {
            if let Some(name) = filename.strip_suffix(".recipe") {
                // Try to get the display name from the recipe header
                let display_name = get_content(&recipe_dir, name)
                    .and_then(|content| parse_recipe(&content))
                    .map_or_else(
                        |_| name.to_string(),
                        |recipe| {
                            let header_name = recipe.header().name();
                            if header_name.is_empty() {
                                name.to_string()
                            } else {
                                header_name.to_string()
                            }
                        },
                    );

                recipes
                    .push(RecipeInfo { name: name.to_string(), display_name });
            }
        }
    }

    Ok(recipes)
}

/// Get the raw content of a recipe file
pub fn get_content(
    recipes_dir: &Path,
    name: &str,
) -> Result<String, RecipeError> {
    let recipe_path = recipes_dir.join(format!("{name}.recipe"));

    if !recipe_path.exists() {
        return Err(RecipeError::NotFound { name: name.to_string() });
    }

    let content = std::fs::read_to_string(recipe_path)?;
    Ok(content)
}

/// Parse and retrieve a recipe by name
pub fn get(recipes_dir: &Path, name: &str) -> Result<Recipe, RecipeError> {
    let content = get_content(recipes_dir, name)?;
    let recipe = parse_recipe(&content)?;

    info!("Retrieved recipe: {recipe:?}");

    Ok(recipe)
}

/// Get the recipes directory path from a config file path
#[must_use]
pub fn get_recipes_dir(config_file_path: &str) -> std::path::PathBuf {
    std::path::Path::new(config_file_path)
        .parent()
        .expect("Config file path should have a parent directory")
        .join("recipes")
}

/// Parse a recipe from its string content
fn parse_recipe(content: &str) -> Result<Recipe, RecipeError> {
    if content.trim().is_empty() {
        return Err(RecipeError::EmptyContent);
    }

    HEADER_REGEX.captures(content).map_or_else(
        || Ok(Recipe::new(Header::empty(), content.to_string())),
        |captures| {
            let header_content =
                captures.get(2).ok_or_else(|| RecipeError::InvalidFormat {
                    message: "Could not extract header content".to_string(),
                })?;
            let body_content =
                captures.get(4).ok_or_else(|| RecipeError::InvalidFormat {
                    message: "Could not extract body content".to_string(),
                })?;

            let header = Header::parse(header_content.as_str())?;
            let body = body_content.as_str().trim().to_string();

            Ok(Recipe::new(header, body))
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recipe_parsing_1() {
        let content =
            "---\nname: Test Recipe\n---\nThis is the body of the recipe.";
        let recipe = super::parse_recipe(content).unwrap();

        assert_eq!(recipe.header.name(), "Test Recipe");
        assert_eq!(recipe.body, "This is the body of the recipe.");
    }

    #[test]
    fn test_recipe_parsing_2() {
        let content =
            "----\nname: Test Recipe\n-----\nThis is the body of the recipe.";
        let recipe = super::parse_recipe(content).unwrap();

        assert_eq!(recipe.header.name(), "Test Recipe");
        assert_eq!(recipe.body, "This is the body of the recipe.");
    }

    #[test]
    fn test_recipe_parsing_2b() {
        let content =
            "---\nname: Test Recipe\n-----\nThis is the body of the recipe.";
        let recipe = super::parse_recipe(content).unwrap();

        assert_eq!(recipe.header.name(), "Test Recipe");
        assert_eq!(recipe.body, "This is the body of the recipe.");
    }

    #[test]
    fn test_recipe_parsing_3() {
        let content = "---\nname: Test Recipe\nDoes not start on new line-----\nThis is the body of the recipe.";
        let recipe = super::parse_recipe(content).unwrap();

        assert_eq!(recipe.header.name(), "");
        assert_eq!(
            recipe.body,
            "---\nname: Test Recipe\nDoes not start on new line-----\nThis is the body of the recipe."
        );
    }

    #[test]
    fn test_recipe_parsing_4() {
        let content = "---\nname: Test Recipe\nanotherParam: some other value\n-----\nThis is the body of the recipe.";
        let recipe = super::parse_recipe(content).unwrap();

        assert_eq!(recipe.header.name(), "Test Recipe");
        assert_eq!(recipe.body, "This is the body of the recipe.");
    }

    #[test]
    fn test_recipe_parsing_5() {
        let content = "----\nname: do\nallowed_tools: ['ls']\n----\nYou are a command-line assistant.\n\nThe user will request you to do something in their command line environment.\n\nYour goal is to respond with the command they should run.\n\n## Examples\n\n<example_1>\nuser: please untar photos.tar.gz\nassistant: tar -xzf archive.tar.gz\n</example_1>\n\n<example_2>\nuser: please untar the file\nassistant: <executes tool `ls *.tar.gz` to see what .tar.gz file exists in the current directory>\ntool: my_file.tar.gz\nassistant: tar -xzf my_file.tar.gz\n</example_2>";
        let recipe = super::parse_recipe(content).unwrap();

        assert_eq!(recipe.header.name(), "do");
        assert_eq!(
            recipe.body,
            "You are a command-line assistant.\n\nThe user will request you to do something in their command line environment.\n\nYour goal is to respond with the command they should run.\n\n## Examples\n\n<example_1>\nuser: please untar photos.tar.gz\nassistant: tar -xzf archive.tar.gz\n</example_1>\n\n<example_2>\nuser: please untar the file\nassistant: <executes tool `ls *.tar.gz` to see what .tar.gz file exists in the current directory>\ntool: my_file.tar.gz\nassistant: tar -xzf my_file.tar.gz\n</example_2>"
        );
    }

    #[test]
    fn test_recipe_parsing_no_header() {
        let content = "This is just a plain recipe body with no header.";
        let recipe = super::parse_recipe(content).unwrap();

        assert_eq!(recipe.header.name(), "");
        assert_eq!(
            recipe.body,
            "This is just a plain recipe body with no header."
        );
    }

    #[test]
    fn test_recipe_parsing_empty_header() {
        let content = "---\n\n---\nThis is the body of the recipe.";
        let recipe = super::parse_recipe(content).unwrap();

        assert_eq!(recipe.header.name(), "");
        assert_eq!(recipe.body, "This is the body of the recipe.");
    }

    #[test]
    fn test_recipe_parsing_empty_body() {
        let content = "---\nname: Test Recipe\n---\n";
        let recipe = super::parse_recipe(content).unwrap();

        assert_eq!(recipe.header.name(), "Test Recipe");
        assert_eq!(recipe.body, "");
    }

    #[test]
    fn test_recipe_parsing_whitespace_only_body() {
        let content = "---\nname: Test Recipe\n---\n   \n  \t  \n";
        let recipe = super::parse_recipe(content).unwrap();

        assert_eq!(recipe.header.name(), "Test Recipe");
        assert_eq!(recipe.body, "");
    }

    #[test]
    fn test_recipe_parsing_only_opening_delimiter() {
        let content =
            "---\nname: Test Recipe\nThis should all be treated as body";
        let recipe = super::parse_recipe(content).unwrap();

        assert_eq!(recipe.header.name(), "");
        assert_eq!(
            recipe.body,
            "---\nname: Test Recipe\nThis should all be treated as body"
        );
    }

    #[test]
    fn test_recipe_parsing_mismatched_delimiters() {
        let content = "---\nname: Test Recipe\n----\nThis is the body.";
        let recipe = super::parse_recipe(content).unwrap();

        assert_eq!(recipe.header.name(), "Test Recipe");
        assert_eq!(recipe.body, "This is the body.");
    }

    #[test]
    fn test_recipe_parsing_many_dashes() {
        let content =
            "----------\nname: Test Recipe\n----------\nThis is the body.";
        let recipe = super::parse_recipe(content).unwrap();

        assert_eq!(recipe.header.name(), "Test Recipe");
        assert_eq!(recipe.body, "This is the body.");
    }

    #[test]
    fn test_recipe_parsing_delimiters_with_extra_whitespace() {
        let content =
            "---   \n  \nname: Test Recipe\n  \n---  \t \nThis is the body.";
        let recipe = super::parse_recipe(content).unwrap();

        // Actually, I don't care too much about untrimmed whitespace in the header:
        // assert_eq!(recipe.header, "  \nname: Test Recipe\n  ");
        assert_eq!(recipe.body, "This is the body.");
    }

    #[test]
    fn test_recipe_parsing_body_with_similar_delimiters() {
        let content = "---\nname: Test Recipe\n---\nHere's some content.\n\n---\nThis looks like a delimiter but it's in the body.\n---\n\nMore content.";
        let recipe = super::parse_recipe(content).unwrap();

        assert_eq!(recipe.header.name(), "Test Recipe");
        assert_eq!(
            recipe.body,
            "Here's some content.\n\n---\nThis looks like a delimiter but it's in the body.\n---\n\nMore content."
        );
    }

    #[test]
    fn test_recipe_parsing_complex_yaml_header() {
        let content = "---\nname: complex recipe\nauthor: test\nversion: 1.0\ntags:\n  - utility\n  - command-line\noptions:\n  verbose: true\n  timeout: 30\n---\nThis is a recipe with a complex YAML header.";
        let recipe = super::parse_recipe(content).unwrap();

        assert_eq!(recipe.header.name(), "complex recipe");
        assert_eq!(
            recipe.body,
            "This is a recipe with a complex YAML header."
        );
    }

    #[test]
    fn test_recipe_parsing_header_with_dashes_in_content() {
        let content = "---\nname: My Recipe\ndescription: This has -- dashes in it\ncommand: ls -la\n---\nBody content here.";
        let recipe = super::parse_recipe(content).unwrap();

        assert_eq!(recipe.header.name(), "My Recipe");
        assert_eq!(recipe.body, "Body content here.");
    }

    #[test]
    fn test_recipe_parsing_minimal_three_dashes() {
        let content = "---\na\n---\nb";
        let recipe = super::parse_recipe(content).unwrap();

        assert_eq!(recipe.header.name(), "");
        assert_eq!(recipe.body, "b");
    }

    #[test]
    fn test_recipe_parsing_unicode_content() {
        let content = "---\nname: 测试食谱\nauthor: José García\n---\nThis recipe contains unicode: café, naïve, 中文";
        let recipe = super::parse_recipe(content).unwrap();

        assert_eq!(recipe.header.name(), "测试食谱");
        assert_eq!(
            recipe.body,
            "This recipe contains unicode: café, naïve, 中文"
        );
    }

    #[test]
    fn test_recipe_parsing_multiline_strings_in_header() {
        let content = "---\nname: Test\ndescription: |\n  This is a multiline\n  description that spans\n  multiple lines\n---\nBody content.";
        let recipe = super::parse_recipe(content).unwrap();

        assert_eq!(recipe.header.name(), "Test");
        assert_eq!(recipe.body, "Body content.");
    }

    #[test]
    fn test_recipe_parsing_proper_yaml() {
        let content = "---\nname: test recipe\nallowed_tools:\n  - ls\n  - cat\n---\nThis is the body.";
        let recipe = super::parse_recipe(content).unwrap();

        assert_eq!(recipe.header.name(), "test recipe");
        assert_eq!(recipe.header.allowed_tools(), &["ls", "cat"]);
        assert_eq!(recipe.body, "This is the body.");
    }

    #[test]
    fn test_recipe_parsing_yaml_with_quotes() {
        let content = "---\nname: \"quoted name\"\nallowed_tools: [\"ls\", \"cat\"]\n---\nBody content.";
        let recipe = super::parse_recipe(content).unwrap();

        assert_eq!(recipe.header.name(), "quoted name");
        assert_eq!(recipe.header.allowed_tools(), &["ls", "cat"]);
        assert_eq!(recipe.body, "Body content.");
    }

    #[test]
    fn test_recipe_error_handling() {
        // Test empty content
        let result = super::parse_recipe("");
        assert!(matches!(result, Err(RecipeError::EmptyContent)));

        // Test whitespace only content
        let result = super::parse_recipe("   \n  \t  \n");
        assert!(matches!(result, Err(RecipeError::EmptyContent)));
    }

    #[test]
    fn test_recipe_info_struct() {
        let info = RecipeInfo {
            name: "test".to_string(),
            display_name: "Test Recipe".to_string(),
        };

        assert_eq!(info.name, "test");
        assert_eq!(info.display_name, "Test Recipe");
    }
}
