use crate::app::prelude::*;

pub(crate) fn run() {
    let outcome = match parse_command(env::args().skip(1).collect()) {
        Ok(Command::Doctor(mode)) => run_doctor(mode),
        Ok(Command::ArchitectureMap) => run_architecture_map(),
        Ok(Command::ClaimAudit) => run_claim_audit(),
        Ok(Command::ReleaseLaneArtifact(lane)) => run_release_lane_artifact(&lane),
        Ok(Command::ReleaseReadiness) => run_release_readiness(),
        Ok(Command::StageReleaseArtifacts { input, output }) => {
            run_stage_release_artifacts(&input, &output)
        }
        Ok(Command::VisualProof(VisualProofCommand::AllReleaseLanes)) => run_visual_proof(),
        Ok(Command::VisualProof(VisualProofCommand::Run { lane, command })) => repo_root()
            .map_err(|message| vec![Finding::new("VISUAL-PROOF", message)])
            .and_then(|root| run_visual_proof_command(&root, &lane, &command)),
        Ok(Command::Help) => {
            print_usage();
            Ok(())
        }
        Err(error) => Err(vec![Finding::new("CLI", error)]),
    };

    match outcome {
        Ok(()) => {}
        Err(findings) => {
            eprintln!("scena doctor failed with {} finding(s):", findings.len());
            for finding in findings {
                eprintln!("- {}: {}", finding.rule, finding.message);
            }
            process::exit(1);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DoctorMode {
    Docs,
    Architecture,
    Full,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Command {
    Doctor(DoctorMode),
    ArchitectureMap,
    ClaimAudit,
    ReleaseLaneArtifact(String),
    ReleaseReadiness,
    StageReleaseArtifacts { input: String, output: String },
    VisualProof(VisualProofCommand),
    Help,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum VisualProofCommand {
    AllReleaseLanes,
    Run { lane: String, command: Vec<String> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Finding {
    pub(crate) rule: &'static str,
    pub(crate) message: String,
}

impl Finding {
    pub(crate) fn new(rule: &'static str, message: impl Into<String>) -> Self {
        let mut message = message.into();
        let reference = finding_reference(rule);
        if !message.contains(reference) {
            message.push_str("; see ");
            message.push_str(reference);
        }
        Self { rule, message }
    }
}

pub(crate) fn finding_reference(rule: &str) -> &'static str {
    if rule.starts_with("RELEASE") || rule.starts_with("CLAIM") || rule.starts_with("M10") {
        "docs/release-notes/v1.0.1.md"
    } else if rule.contains("STATE-OF-ART")
        || rule == "ARCH-RENDER-TRUTH"
        || rule == "ARCH-RENDER-STANDARD-MATH"
        || rule == "ARCH-RENDER-WORLD-BAKE"
        || rule == "BINARY-ASSET-TRUTH-P9"
    {
        "docs/rendering.md"
    } else if rule.contains("M8") || rule.contains("GLTF") || rule.contains("ASSETS") {
        "docs/assets.md"
    } else if rule.contains("M7") || rule.contains("ERGONOMICS") {
        "docs/api.md"
    } else if rule.contains("VISUAL") || rule.contains("SCREENSHOT") {
        "docs/headless-rendering.md"
    } else if rule.contains("PLATFORM") || rule.contains("BACKEND") || rule.contains("WEBGL") {
        "docs/platforms.md"
    } else if rule.contains("PREPARE") || rule.contains("LIFECYCLE") {
        "docs/lifecycle.md"
    } else if rule.starts_with("ARCH-CONTRACT")
        || rule.starts_with("ARCH-DEPENDENCY-DIRECTION")
        || rule.starts_with("ARCH-PUBLIC-API-OWNERSHIP")
        || rule.starts_with("ARCH-VIEWER-FACADE")
        || rule.starts_with("ARCH-RENDER-SINGLETON")
        || rule == "ARCH-XTASK-SPLIT"
        || rule == "ARCHITECTURE-MAP"
    {
        "docs/api.md"
    } else {
        "docs/README.md"
    }
}

pub(crate) fn parse_command(args: Vec<String>) -> Result<Command, String> {
    if args.is_empty() || args == ["--help"] || args == ["-h"] {
        return Ok(Command::Help);
    }

    if args.first().map(String::as_str) == Some("claim-audit") {
        if args.len() == 1 {
            return Ok(Command::ClaimAudit);
        }
        return Err("claim-audit accepts no arguments".to_string());
    }

    if args.first().map(String::as_str) == Some("architecture-map") {
        if args.len() == 1 {
            return Ok(Command::ArchitectureMap);
        }
        return Err("architecture-map accepts no arguments".to_string());
    }

    if args.first().map(String::as_str) == Some("release-lane-artifact") {
        if args.len() == 2 {
            return Ok(Command::ReleaseLaneArtifact(args[1].clone()));
        }
        return Err("release-lane-artifact expects exactly one lane argument".to_string());
    }

    if args.first().map(String::as_str) == Some("release-readiness") {
        if args.len() == 1 {
            return Ok(Command::ReleaseReadiness);
        }
        return Err("release-readiness accepts no arguments".to_string());
    }

    if args.first().map(String::as_str) == Some("stage-release-artifacts") {
        if args.len() == 3 {
            return Ok(Command::StageReleaseArtifacts {
                input: args[1].clone(),
                output: args[2].clone(),
            });
        }
        return Err(
            "stage-release-artifacts expects <downloaded-root> <canonical-output-root>".to_string(),
        );
    }

    if args.first().map(String::as_str) == Some("visual-proof") {
        if args.len() == 2 && args[1] == "--all-release-lanes" {
            return Ok(Command::VisualProof(VisualProofCommand::AllReleaseLanes));
        }
        if args.len() >= 4 && args[2] == "--" {
            return Ok(Command::VisualProof(VisualProofCommand::Run {
                lane: args[1].clone(),
                command: args[3..].to_vec(),
            }));
        }
        return Err(
            "visual-proof expects '--all-release-lanes' or '<lane> -- <command...>'".to_string(),
        );
    }

    if args.first().map(String::as_str) != Some("doctor") {
        return Err(format!(
            "unknown command '{}'; expected 'doctor', 'architecture-map', 'claim-audit', 'release-lane-artifact', 'release-readiness', 'stage-release-artifacts', or 'visual-proof'",
            args.first().map(String::as_str).unwrap_or("")
        ));
    }

    let mode = match args.get(1).map(String::as_str) {
        None | Some("--full") => DoctorMode::Full,
        Some("--docs") => DoctorMode::Docs,
        Some("--architecture") => DoctorMode::Architecture,
        Some("--help") | Some("-h") => return Ok(Command::Help),
        Some(other) => {
            return Err(format!(
                "unknown doctor mode '{other}'; expected --docs, --architecture, or --full"
            ));
        }
    };

    if args.len() > 2 {
        return Err("doctor accepts at most one mode flag".to_string());
    }

    Ok(Command::Doctor(mode))
}

pub(crate) fn print_usage() {
    println!(
        "Usage:\n  cargo run -p xtask -- doctor --docs\n  cargo run -p xtask -- doctor --architecture\n  cargo run -p xtask -- doctor --full\n  cargo run -p xtask -- architecture-map\n  cargo run -p xtask -- claim-audit\n  cargo run -p xtask -- release-lane-artifact <lane>\n  cargo run -p xtask -- release-readiness\n  cargo run -p xtask -- stage-release-artifacts <downloaded-root> <canonical-output-root>\n  cargo run -p xtask -- visual-proof --all-release-lanes\n  cargo run -p xtask -- visual-proof <lane> -- <command...>"
    );
}
