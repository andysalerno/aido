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

    #[test]
    fn test_recipe_parsing_no_header() {
        let content = "This is just a plain recipe body with no header.";
        let recipe = super::parse_recipe(content).unwrap();

        assert_eq!(recipe.header, "");
        assert_eq!(
            recipe.body,
            "This is just a plain recipe body with no header."
        );
    }

    #[test]
    fn test_recipe_parsing_empty_header() {
        let content = "---\n\n---\nThis is the body of the recipe.";
        let recipe = super::parse_recipe(content).unwrap();

        assert_eq!(recipe.header, "");
        assert_eq!(recipe.body, "This is the body of the recipe.");
    }

    #[test]
    fn test_recipe_parsing_empty_body() {
        let content = "---\ntitle: Test Recipe\n---\n";
        let recipe = super::parse_recipe(content).unwrap();

        assert_eq!(recipe.header, "title: Test Recipe");
        assert_eq!(recipe.body, "");
    }

    #[test]
    fn test_recipe_parsing_whitespace_only_body() {
        let content = "---\ntitle: Test Recipe\n---\n   \n  \t  \n";
        let recipe = super::parse_recipe(content).unwrap();

        assert_eq!(recipe.header, "title: Test Recipe");
        assert_eq!(recipe.body, "");
    }

    #[test]
    fn test_recipe_parsing_only_opening_delimiter() {
        let content =
            "---\ntitle: Test Recipe\nThis should all be treated as body";
        let recipe = super::parse_recipe(content).unwrap();

        assert_eq!(recipe.header, "");
        assert_eq!(
            recipe.body,
            "---\ntitle: Test Recipe\nThis should all be treated as body"
        );
    }

    #[test]
    fn test_recipe_parsing_mismatched_delimiters() {
        let content = "---\ntitle: Test Recipe\n----\nThis is the body.";
        let recipe = super::parse_recipe(content).unwrap();

        assert_eq!(recipe.header, "title: Test Recipe");
        assert_eq!(recipe.body, "This is the body.");
    }

    #[test]
    fn test_recipe_parsing_many_dashes() {
        let content =
            "----------\ntitle: Test Recipe\n----------\nThis is the body.";
        let recipe = super::parse_recipe(content).unwrap();

        assert_eq!(recipe.header, "title: Test Recipe");
        assert_eq!(recipe.body, "This is the body.");
    }

    #[test]
    fn test_recipe_parsing_delimiters_with_extra_whitespace() {
        let content =
            "---   \n  \ntitle: Test Recipe\n  \n---  \t \nThis is the body.";
        let recipe = super::parse_recipe(content).unwrap();

        // Actually, I don't care too much about untrimmed whitespace in the header:
        // assert_eq!(recipe.header, "  \ntitle: Test Recipe\n  ");
        assert_eq!(recipe.body, "This is the body.");
    }

    #[test]
    fn test_recipe_parsing_body_with_similar_delimiters() {
        let content = "---\ntitle: Test Recipe\n---\nHere's some content.\n\n---\nThis looks like a delimiter but it's in the body.\n---\n\nMore content.";
        let recipe = super::parse_recipe(content).unwrap();

        assert_eq!(recipe.header, "title: Test Recipe");
        assert_eq!(
            recipe.body,
            "Here's some content.\n\n---\nThis looks like a delimiter but it's in the body.\n---\n\nMore content."
        );
    }

    #[test]
    fn test_recipe_parsing_complex_yaml_header() {
        let content = "---\nname: complex recipe\nauthor: test\nversion: 1.0\ntags:\n  - utility\n  - command-line\noptions:\n  verbose: true\n  timeout: 30\n---\nThis is a recipe with a complex YAML header.";
        let recipe = super::parse_recipe(content).unwrap();

        assert_eq!(
            recipe.header,
            "name: complex recipe\nauthor: test\nversion: 1.0\ntags:\n  - utility\n  - command-line\noptions:\n  verbose: true\n  timeout: 30"
        );
        assert_eq!(
            recipe.body,
            "This is a recipe with a complex YAML header."
        );
    }

    #[test]
    fn test_recipe_parsing_header_with_dashes_in_content() {
        let content = "---\ntitle: My Recipe\ndescription: This has -- dashes in it\ncommand: ls -la\n---\nBody content here.";
        let recipe = super::parse_recipe(content).unwrap();

        assert_eq!(
            recipe.header,
            "title: My Recipe\ndescription: This has -- dashes in it\ncommand: ls -la"
        );
        assert_eq!(recipe.body, "Body content here.");
    }

    #[test]
    fn test_recipe_parsing_minimal_three_dashes() {
        let content = "---\na\n---\nb";
        let recipe = super::parse_recipe(content).unwrap();

        assert_eq!(recipe.header, "a");
        assert_eq!(recipe.body, "b");
    }

    #[test]
    fn test_recipe_parsing_unicode_content() {
        let content = "---\ntitle: 测试食谱\nauthor: José García\n---\nThis recipe contains unicode: café, naïve, 中文";
        let recipe = super::parse_recipe(content).unwrap();

        assert_eq!(recipe.header, "title: 测试食谱\nauthor: José García");
        assert_eq!(
            recipe.body,
            "This recipe contains unicode: café, naïve, 中文"
        );
    }

    #[test]
    fn test_recipe_parsing_multiline_strings_in_header() {
        let content = "---\ntitle: Test\ndescription: |\n  This is a multiline\n  description that spans\n  multiple lines\n---\nBody content.";
        let recipe = super::parse_recipe(content).unwrap();

        assert_eq!(
            recipe.header,
            "title: Test\ndescription: |\n  This is a multiline\n  description that spans\n  multiple lines"
        );
        assert_eq!(recipe.body, "Body content.");
    }
}
