use std::path::Path;

use log::info;

#[derive(Debug, Clone)]
pub struct Recipe {
    /// The yaml frontmatter header of the recipe
    header: String,

    /// The body of the recipe
    body: String,
}

/// Lists all available recipes in the recipes directory
pub fn list(config_file_path: &str) -> Result<(), Box<dyn std::error::Error>> {
    // recipe dir is in the parent dir of the config file
    let recipe_dir = get_recipes_dir(config_file_path);

    // List all recipes in the directory:
    let entries = std::fs::read_dir(recipe_dir)?;

    for entry in entries
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|ft| ft.is_file()))
    {
        if let Some(name) = entry.file_name().to_str()
            && name.ends_with(".recipe")
        {
            println!("- {name}");
        }
    }

    // todo - this should obviously return the recipes, not just print them
    Ok(())
}

pub fn get_content(
    recipes_dir: &Path,
    name: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let recipe_path = recipes_dir.join(format!("{name}.recipe"));

    if !recipe_path.exists() {
        return Err(format!("Recipe '{name}' not found").into());
    }

    let content = std::fs::read_to_string(recipe_path)?;
    Ok(content)
}

fn parse_recipe(content: &str) -> Result<Recipe, Box<dyn std::error::Error>> {
    use regex::Regex;

    if content.trim().is_empty() {
        return Err("Recipe content is empty".into());
    }

    // Regex pattern to match header delimiters (3 or more dashes) at the start of the document
    // The pattern captures:
    // 1. Opening delimiter (3+ dashes) at the very beginning
    // 2. Header content (non-greedy match, including newlines)
    // 3. Closing delimiter (3+ dashes)
    // 4. Remaining body content
    let header_regex =
        Regex::new(r"(?s)^(-{3,})\s*\n(.*?)\n(-{3,})\s*\n(.*)$").unwrap();

    header_regex.captures(content).map_or_else(
        || Ok(Recipe { header: String::new(), body: content.to_string() }),
        |captures| {
            let header = captures.get(2).unwrap().as_str();
            let body = captures.get(4).unwrap().as_str();

            Ok(Recipe {
                header: header.to_string(),
                body: body.trim().to_string(),
            })
        },
    )
}

pub fn get(
    recipes_dir: &Path,
    name: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let content = get_content(recipes_dir, name)?;
    let recipe = parse_recipe(&content)?;

    info!("Retrieved recipe: {recipe:?}");

    // Return the body of the recipe
    Ok(recipe.body)
}

pub fn get_recipes_dir(config_file_path: &str) -> std::path::PathBuf {
    std::path::Path::new(config_file_path).parent().unwrap().join("recipes")
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_recipe_parsing_1() {
        let content =
            "---\ntitle: Test Recipe\n---\nThis is the body of the recipe.";
        let recipe = super::parse_recipe(content).unwrap();

        assert_eq!(recipe.header, "title: Test Recipe");
        assert_eq!(recipe.body, "This is the body of the recipe.");
    }

    #[test]
    fn test_recipe_parsing_2() {
        let content =
            "----\ntitle: Test Recipe\n-----\nThis is the body of the recipe.";
        let recipe = super::parse_recipe(content).unwrap();

        assert_eq!(recipe.header, "title: Test Recipe");
        assert_eq!(recipe.body, "This is the body of the recipe.");
    }

    #[test]
    fn test_recipe_parsing_2b() {
        let content =
            "---\ntitle: Test Recipe\n-----\nThis is the body of the recipe.";
        let recipe = super::parse_recipe(content).unwrap();

        assert_eq!(recipe.header, "title: Test Recipe");
        assert_eq!(recipe.body, "This is the body of the recipe.");
    }

    #[test]
    fn test_recipe_parsing_3() {
        let content = "---\ntitle: Test Recipe\nDoes not start on new line-----\nThis is the body of the recipe.";
        let recipe = super::parse_recipe(content).unwrap();

        assert_eq!(recipe.header, "");
        assert_eq!(
            recipe.body,
            "---\ntitle: Test Recipe\nDoes not start on new line-----\nThis is the body of the recipe."
        );
    }

    #[test]
    fn test_recipe_parsing_4() {
        let content = "---\ntitle: Test Recipe\nanotherParam: some other value\n-----\nThis is the body of the recipe.";
        let recipe = super::parse_recipe(content).unwrap();

        assert_eq!(
            recipe.header,
            "title: Test Recipe\nanotherParam: some other value"
        );
        assert_eq!(recipe.body, "This is the body of the recipe.");
    }

    #[test]
    fn test_recipe_parsing_5() {
        let content = "----\nname: do\nallowed_tools: ['ls']\n----\nYou are a command-line assistant.\n\nThe user will request you to do something in their command line environment.\n\nYour goal is to respond with the command they should run.\n\n## Examples\n\n<example_1>\nuser: please untar photos.tar.gz\nassistant: tar -xzf archive.tar.gz\n</example_1>\n\n<example_2>\nuser: please untar the file\nassistant: <executes tool `ls *.tar.gz` to see what .tar.gz file exists in the current directory>\ntool: my_file.tar.gz\nassistant: tar -xzf my_file.tar.gz\n</example_2>";
        let recipe = super::parse_recipe(content).unwrap();

        assert_eq!(recipe.header, "name: do\nallowed_tools: ['ls']");
        assert_eq!(
            recipe.body,
            "You are a command-line assistant.\n\nThe user will request you to do something in their command line environment.\n\nYour goal is to respond with the command they should run.\n\n## Examples\n\n<example_1>\nuser: please untar photos.tar.gz\nassistant: tar -xzf archive.tar.gz\n</example_1>\n\n<example_2>\nuser: please untar the file\nassistant: <executes tool `ls *.tar.gz` to see what .tar.gz file exists in the current directory>\ntool: my_file.tar.gz\nassistant: tar -xzf my_file.tar.gz\n</example_2>"
        );
    }
}
