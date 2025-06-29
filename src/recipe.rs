use std::path::Path;

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

    Ok(())
}

pub fn get(
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

pub fn get_recipes_dir(config_file_path: &str) -> std::path::PathBuf {
    std::path::Path::new(config_file_path).parent().unwrap().join("recipes")
}
