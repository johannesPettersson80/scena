use std::path::PathBuf;

#[cfg(feature = "inspection")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DoctorCommandArgs {
    pub(crate) input: String,
}

#[cfg(feature = "inspection")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RenderCommandArgs {
    pub(crate) input: String,
    pub(crate) out: PathBuf,
    pub(crate) width: Option<u32>,
    pub(crate) height: Option<u32>,
    pub(crate) detail: bool,
    pub(crate) gpu: bool,
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
pub(crate) struct RepairCommandArgs {
    pub(crate) input: String,
    pub(crate) from: PathBuf,
    pub(crate) iteration_budget: u32,
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
        })
    }
}

#[cfg(feature = "inspection")]
impl DoctorCommandArgs {
    pub(crate) fn parse(args: &[String]) -> Result<Self, String> {
        let Some(input) = args.first() else {
            return Err(doctor_usage());
        };
        let mut index = 1;
        while index < args.len() {
            match args[index].as_str() {
                "--json" => {
                    index += 1;
                }
                flag => return Err(format!("unknown doctor flag '{flag}'; {}", doctor_usage())),
            }
        }
        Ok(Self {
            input: input.clone(),
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
        let mut gpu = super::scena_input::gpu_requested_from_env();

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
                "--gpu" => {
                    gpu = true;
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
            gpu,
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
impl RepairCommandArgs {
    pub(crate) fn parse(args: &[String]) -> Result<Self, String> {
        let Some(input) = args.first() else {
            return Err(repair_usage());
        };
        let mut from = None;
        let mut iteration_budget = 3;

        let mut index = 1;
        while index < args.len() {
            match args[index].as_str() {
                "--from" => {
                    from = Some(PathBuf::from(flag_value(args, index, "--from")?));
                    index += 2;
                }
                "--iteration-budget" => {
                    iteration_budget = parse_u32(
                        "--iteration-budget",
                        flag_value(args, index, "--iteration-budget")?,
                    )?;
                    index += 2;
                }
                "--json" => {
                    index += 1;
                }
                flag => {
                    return Err(format!("unknown repair flag '{flag}'; {}", repair_usage()));
                }
            }
        }

        Ok(Self {
            input: input.clone(),
            from: from
                .ok_or_else(|| format!("missing --from <report.json>; {}", repair_usage()))?,
            iteration_budget,
        })
    }
}

#[cfg(feature = "inspection")]
fn flag_value(args: &[String], index: usize, flag: &str) -> Result<String, String> {
    flag_value_any(args, index, flag)
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

#[cfg(feature = "inspection")]
fn parse_positive_u32(flag: &str, value: String) -> Result<u32, String> {
    let parsed = parse_u32(flag, value)?;
    if parsed == 0 {
        return Err(format!("{flag} requires a positive integer, got 0"));
    }
    Ok(parsed)
}

#[cfg(feature = "inspection")]
fn parse_u32(flag: &str, value: String) -> Result<u32, String> {
    let parsed = value
        .parse::<u32>()
        .map_err(|_| format!("{flag} requires an unsigned integer, got '{value}'"))?;
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

fn place_usage() -> String {
    "usage: scena place <recipe.json> --import <id> --verb <center|ground|fit_to_size|look_at|align_to_anchor|place_on> [--target x,y,z] [--up x,y,z] [--ground-y y] [--min-size n] [--max-size n] [--target-import id] [--source-anchor name|--source-connector name] [--target-anchor name|--target-connector name]"
        .to_string()
}

#[cfg(feature = "inspection")]
fn render_usage() -> String {
    "usage: scena render <asset-or-recipe> --introspect --out <png> [--gpu] [--width <px>] [--height <px>] [--detail] [--round-floats <0..6>]"
        .to_string()
}

#[cfg(feature = "inspection")]
fn inspect_usage() -> String {
    "usage: scena inspect <asset-or-recipe> [--width <px>] [--height <px>] [--round-floats <0..6>]"
        .to_string()
}

#[cfg(feature = "inspection")]
fn diagnose_usage() -> String {
    "usage: scena diagnose <asset-or-recipe> --visibility [--handle <u64>] [--width <px>] [--height <px>] [--detail] [--round-floats <0..6>]"
        .to_string()
}

#[cfg(feature = "inspection")]
fn doctor_usage() -> String {
    "usage: scena doctor <asset-or-recipe> [--json] [--round-floats <0..6>]".to_string()
}

#[cfg(feature = "inspection")]
fn repair_usage() -> String {
    "usage: scena repair <asset-or-recipe> --from <diagnosis-or-introspection.json> [--iteration-budget <n>] [--round-floats <0..6>]"
        .to_string()
}
