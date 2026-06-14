use std::path::PathBuf;

#[cfg(feature = "inspection")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RenderCommandArgs {
    pub(crate) input: String,
    pub(crate) out: PathBuf,
    pub(crate) width: Option<u32>,
    pub(crate) height: Option<u32>,
    pub(crate) detail: bool,
}

#[cfg(feature = "inspection")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DiagnoseCommandArgs {
    pub(crate) input: String,
    pub(crate) handle: Option<u64>,
    pub(crate) width: Option<u32>,
    pub(crate) height: Option<u32>,
    pub(crate) detail: bool,
}

#[cfg(feature = "inspection")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InspectCommandArgs {
    pub(crate) input: String,
    pub(crate) width: Option<u32>,
    pub(crate) height: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ValidateRecipeCommandArgs {
    pub(crate) recipe: PathBuf,
}

impl ValidateRecipeCommandArgs {
    pub(crate) fn parse(args: &[String]) -> Result<Self, String> {
        let Some(recipe) = args.first() else {
            return Err(validate_recipe_usage());
        };
        if args.len() > 1 {
            return Err(format!(
                "unknown validate-recipe argument '{}'; {}",
                args[1],
                validate_recipe_usage()
            ));
        }
        Ok(Self {
            recipe: PathBuf::from(recipe),
        })
    }
}

#[cfg(feature = "inspection")]
impl RenderCommandArgs {
    pub(crate) fn parse(args: &[String]) -> Result<Self, String> {
        let Some(input) = args.first() else {
            return Err(render_usage());
        };
        let mut introspect = false;
        let mut out = None;
        let mut width = None;
        let mut height = None;
        let mut detail = false;

        let mut index = 1;
        while index < args.len() {
            match args[index].as_str() {
                "--introspect" => {
                    introspect = true;
                    index += 1;
                }
                "--out" => {
                    let value = flag_value(args, index, "--out")?;
                    out = Some(PathBuf::from(value));
                    index += 2;
                }
                "--width" => {
                    width = Some(parse_positive_u32(
                        "--width",
                        flag_value(args, index, "--width")?,
                    )?);
                    index += 2;
                }
                "--height" => {
                    height = Some(parse_positive_u32(
                        "--height",
                        flag_value(args, index, "--height")?,
                    )?);
                    index += 2;
                }
                "--detail" => {
                    detail = true;
                    index += 1;
                }
                "--json" => {
                    index += 1;
                }
                flag => return Err(format!("unknown render flag '{flag}'; {}", render_usage())),
            }
        }

        if !introspect {
            return Err(format!("missing --introspect; {}", render_usage()));
        }
        let out = out.ok_or_else(|| format!("missing --out <png>; {}", render_usage()))?;

        Ok(Self {
            input: input.clone(),
            out,
            width,
            height,
            detail,
        })
    }
}

#[cfg(feature = "inspection")]
impl InspectCommandArgs {
    pub(crate) fn parse(args: &[String]) -> Result<Self, String> {
        let Some(input) = args.first() else {
            return Err(inspect_usage());
        };
        let mut width = None;
        let mut height = None;

        let mut index = 1;
        while index < args.len() {
            match args[index].as_str() {
                "--width" => {
                    width = Some(parse_positive_u32(
                        "--width",
                        flag_value(args, index, "--width")?,
                    )?);
                    index += 2;
                }
                "--height" => {
                    height = Some(parse_positive_u32(
                        "--height",
                        flag_value(args, index, "--height")?,
                    )?);
                    index += 2;
                }
                "--json" => {
                    index += 1;
                }
                flag => {
                    return Err(format!(
                        "unknown inspect flag '{flag}'; {}",
                        inspect_usage()
                    ));
                }
            }
        }

        Ok(Self {
            input: input.clone(),
            width,
            height,
        })
    }
}

#[cfg(feature = "inspection")]
impl DiagnoseCommandArgs {
    pub(crate) fn parse(args: &[String]) -> Result<Self, String> {
        let Some(input) = args.first() else {
            return Err(diagnose_usage());
        };
        let mut visibility = false;
        let mut handle = None;
        let mut width = None;
        let mut height = None;
        let mut detail = false;

        let mut index = 1;
        while index < args.len() {
            match args[index].as_str() {
                "--visibility" => {
                    visibility = true;
                    index += 1;
                }
                "--handle" => {
                    handle = Some(parse_u64("--handle", flag_value(args, index, "--handle")?)?);
                    index += 2;
                }
                "--width" => {
                    width = Some(parse_positive_u32(
                        "--width",
                        flag_value(args, index, "--width")?,
                    )?);
                    index += 2;
                }
                "--height" => {
                    height = Some(parse_positive_u32(
                        "--height",
                        flag_value(args, index, "--height")?,
                    )?);
                    index += 2;
                }
                "--detail" => {
                    detail = true;
                    index += 1;
                }
                "--json" => {
                    index += 1;
                }
                flag => {
                    return Err(format!(
                        "unknown diagnose flag '{flag}'; {}",
                        diagnose_usage()
                    ));
                }
            }
        }

        if !visibility {
            return Err(format!("missing --visibility; {}", diagnose_usage()));
        }

        Ok(Self {
            input: input.clone(),
            handle,
            width,
            height,
            detail,
        })
    }
}

#[cfg(feature = "inspection")]
fn flag_value(args: &[String], index: usize, flag: &str) -> Result<String, String> {
    args.get(index + 1)
        .cloned()
        .ok_or_else(|| format!("{flag} requires a value"))
}

#[cfg(feature = "inspection")]
fn parse_positive_u32(flag: &str, value: String) -> Result<u32, String> {
    let parsed = value
        .parse::<u32>()
        .map_err(|_| format!("{flag} requires a positive integer, got '{value}'"))?;
    if parsed == 0 {
        return Err(format!("{flag} requires a positive integer, got 0"));
    }
    Ok(parsed)
}

#[cfg(feature = "inspection")]
fn parse_u64(flag: &str, value: String) -> Result<u64, String> {
    value
        .parse::<u64>()
        .map_err(|_| format!("{flag} requires an unsigned integer, got '{value}'"))
}

fn validate_recipe_usage() -> String {
    "usage: scena validate-recipe <recipe.json>".to_string()
}

#[cfg(feature = "inspection")]
fn render_usage() -> String {
    "usage: scena render <asset-or-recipe> --introspect --out <png> [--width <px>] [--height <px>] [--detail]"
        .to_string()
}

#[cfg(feature = "inspection")]
fn inspect_usage() -> String {
    "usage: scena inspect <asset-or-recipe> [--width <px>] [--height <px>]".to_string()
}

#[cfg(feature = "inspection")]
fn diagnose_usage() -> String {
    "usage: scena diagnose <asset-or-recipe> --visibility [--handle <u64>] [--width <px>] [--height <px>] [--detail]"
        .to_string()
}
