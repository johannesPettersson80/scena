use crate::app::prelude::*;

pub(crate) fn run_doctor(mode: DoctorMode) -> Result<(), Vec<Finding>> {
    let root = repo_root().map_err(|message| vec![Finding::new("DOCTOR-ROOT", message)])?;
    let started = std::time::Instant::now();
    let (findings, source_metrics) = with_source_cache_profiled(&root, || {
        let mut findings = Vec::new();
        match mode {
            DoctorMode::Docs => run_docs_doctor(&root, &mut findings),
            DoctorMode::Architecture => run_architecture_doctor(&root, &mut findings),
            DoctorMode::Full => {
                run_docs_doctor(&root, &mut findings);
                run_architecture_doctor(&root, &mut findings);
            }
        }
        findings
    });
    eprintln!(
        "scena doctor source-cache: file_opens={} bytes_read={} source_tree_walks={} directory_reads={} cache_hits={} elapsed_ms={}",
        source_metrics.file_opens,
        source_metrics.bytes_read,
        source_metrics.source_tree_walks,
        source_metrics.directory_reads,
        source_metrics.cache_hits,
        started.elapsed().as_millis(),
    );

    if findings.is_empty() {
        println!("scena doctor: mode={mode:?} status=pass");
        Ok(())
    } else {
        Err(findings)
    }
}
