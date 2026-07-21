use std::path::PathBuf;

use super::scena_policy::push_allow_root;

#[cfg(feature = "inspection")]
#[path = "args/inspection.rs"]
mod inspection;
#[cfg(feature = "inspection")]
pub(crate) use inspection::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ValidateRecipeCommandArgs {
    pub(crate) recipe: PathBuf,
    pub(crate) max_imports: Option<usize>,
    pub(crate) syntax_only: bool,
    pub(crate) allow_roots: Vec<PathBuf>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PlaceCommandArgs {
    pub(crate) recipe: PathBuf,
    pub(crate) import_id: String,
    pub(crate) verb: String,
    pub(crate) target: Option<scena::Vec3>,
    pub(crate) up: Option<scena::Vec3>,
    pub(crate) ground_y: Option<f32>,
    pub(crate) min_size: Option<f32>,
    pub(crate) max_size: Option<f32>,
    pub(crate) target_import_id: Option<String>,
    pub(crate) source_anchor: Option<String>,
    pub(crate) target_anchor: Option<String>,
    pub(crate) source_connector: Option<String>,
    pub(crate) target_connector: Option<String>,
    pub(crate) apply: bool,
    pub(crate) expected_source_sha256: Option<String>,
}

impl ValidateRecipeCommandArgs {
    pub(crate) fn parse(args: &[String]) -> Result<Self, String> {
        let Some(recipe) = args.first() else {
            return Err(validate_recipe_usage());
        };
        let mut max_imports = None;
        let mut syntax_only = false;
        let mut allow_roots = Vec::new();
        let mut index = 1;
        while index < args.len() {
            match args[index].as_str() {
                "--max-imports" => {
                    max_imports = Some(parse_positive_usize(
                        "--max-imports",
                        flag_value_any(args, index, "--max-imports")?,
                    )?);
                    index += 2;
                }
                "--json" => {
                    index += 1;
                }
                "--syntax-only" => {
                    syntax_only = true;
                    index += 1;
                }
                "--full" => {
                    syntax_only = false;
                    index += 1;
                }
                "--allow-root" => {
                    push_allow_root(args, index, &mut allow_roots)?;
                    index += 2;
                }
                flag => {
                    return Err(format!(
                        "unknown validate-recipe argument '{flag}'; {}",
                        validate_recipe_usage()
                    ));
                }
            }
        }
        Ok(Self {
            recipe: PathBuf::from(recipe),
            max_imports,
            syntax_only,
            allow_roots,
        })
    }
}

impl PlaceCommandArgs {
    pub(crate) fn parse(args: &[String]) -> Result<Self, String> {
        let Some(recipe) = args.first() else {
            return Err(place_usage());
        };
        let mut import_id = None;
        let mut verb = None;
        let mut target = None;
        let mut up = None;
        let mut ground_y = None;
        let mut min_size = None;
        let mut max_size = None;
        let mut target_import_id = None;
        let mut source_anchor = None;
        let mut target_anchor = None;
        let mut source_connector = None;
        let mut target_connector = None;
        let mut apply = false;
        let mut expected_source_sha256 = None;

        let mut index = 1;
        while index < args.len() {
            match args[index].as_str() {
                "--import" => {
                    import_id = Some(flag_value_any(args, index, "--import")?);
                    index += 2;
                }
                "--verb" => {
                    verb = Some(flag_value_any(args, index, "--verb")?);
                    index += 2;
                }
                "--target" => {
                    target = Some(parse_vec3(
                        "--target",
                        flag_value_any(args, index, "--target")?,
                    )?);
                    index += 2;
                }
                "--up" => {
                    up = Some(parse_vec3("--up", flag_value_any(args, index, "--up")?)?);
                    index += 2;
                }
                "--ground-y" => {
                    ground_y = Some(parse_f32(
                        "--ground-y",
                        flag_value_any(args, index, "--ground-y")?,
                    )?);
                    index += 2;
                }
                "--min-size" => {
                    min_size = Some(parse_f32(
                        "--min-size",
                        flag_value_any(args, index, "--min-size")?,
                    )?);
                    index += 2;
                }
                "--max-size" => {
                    max_size = Some(parse_f32(
                        "--max-size",
                        flag_value_any(args, index, "--max-size")?,
                    )?);
                    index += 2;
                }
                "--target-import" => {
                    target_import_id = Some(flag_value_any(args, index, "--target-import")?);
                    index += 2;
                }
                "--source-anchor" => {
                    source_anchor = Some(flag_value_any(args, index, "--source-anchor")?);
                    index += 2;
                }
                "--target-anchor" => {
                    target_anchor = Some(flag_value_any(args, index, "--target-anchor")?);
                    index += 2;
                }
                "--source-connector" => {
                    source_connector = Some(flag_value_any(args, index, "--source-connector")?);
                    index += 2;
                }
                "--target-connector" => {
                    target_connector = Some(flag_value_any(args, index, "--target-connector")?);
                    index += 2;
                }
                "--apply" => {
                    apply = true;
                    index += 1;
                }
                "--expect-source-sha256" => {
                    let digest = flag_value_any(args, index, "--expect-source-sha256")?;
                    if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                        return Err(
                            "--expect-source-sha256 requires exactly 64 hexadecimal characters"
                                .to_owned(),
                        );
                    }
                    expected_source_sha256 = Some(digest.to_ascii_lowercase());
                    index += 2;
                }
                "--json" => {
                    index += 1;
                }
                flag => return Err(format!("unknown place flag '{flag}'; {}", place_usage())),
            }
        }

        Ok(Self {
            recipe: PathBuf::from(recipe),
            import_id: import_id
                .ok_or_else(|| format!("missing --import <id>; {}", place_usage()))?,
            verb: verb.ok_or_else(|| format!("missing --verb <verb>; {}", place_usage()))?,
            target,
            up,
            ground_y,
            min_size,
            max_size,
            target_import_id,
            source_anchor,
            target_anchor,
            source_connector,
            target_connector,
            apply,
            expected_source_sha256,
        })
    }
}

fn flag_value_any(args: &[String], index: usize, flag: &str) -> Result<String, String> {
    args.get(index + 1)
        .cloned()
        .ok_or_else(|| format!("{flag} requires a value"))
}

fn parse_f32(flag: &str, value: String) -> Result<f32, String> {
    let parsed = value
        .parse::<f32>()
        .map_err(|_| format!("{flag} requires a finite number, got '{value}'"))?;
    if !parsed.is_finite() {
        return Err(format!("{flag} requires a finite number, got '{value}'"));
    }
    Ok(parsed)
}

fn parse_vec3(flag: &str, value: String) -> Result<scena::Vec3, String> {
    let parts = value
        .split([',', ' '])
        .filter(|part| !part.is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if parts.len() != 3 {
        return Err(format!(
            "{flag} requires three comma- or space-separated numbers"
        ));
    }
    Ok(scena::Vec3::new(
        parse_f32(flag, parts[0].clone())?,
        parse_f32(flag, parts[1].clone())?,
        parse_f32(flag, parts[2].clone())?,
    ))
}

fn parse_positive_usize(flag: &str, value: String) -> Result<usize, String> {
    let parsed = value
        .parse::<usize>()
        .map_err(|_| format!("{flag} requires an unsigned integer, got '{value}'"))?;
    if parsed == 0 {
        return Err(format!("{flag} requires a positive integer, got 0"));
    }
    Ok(parsed)
}

fn validate_recipe_usage() -> String {
    "usage: scena validate-recipe <recipe.json> [--full|--syntax-only] [--max-imports <n>] [--allow-root <directory>]..."
        .to_string()
}

fn place_usage() -> String {
    "usage: scena place <recipe.json> --import <id> --verb <center|ground|fit_to_size|look_at|align_to_anchor|place_on> [--target x,y,z] [--up x,y,z] [--ground-y y] [--min-size n] [--max-size n] [--target-import id] [--source-anchor name|--source-connector name] [--target-anchor name|--target-connector name] [--apply] [--expect-source-sha256 <hex>]"
        .to_string()
}
