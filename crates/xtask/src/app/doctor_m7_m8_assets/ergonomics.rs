use crate::app::prelude::*;
mod examples;
mod foundation;
mod inspection;
mod visual_browser;
use examples::check_m7_example_contracts;
use foundation::check_m7_foundation_contracts;
use inspection::check_m7_inspection_contracts;
use visual_browser::check_m7_visual_browser_contracts;

pub(crate) fn check_m7_ergonomics_contracts(root: &Path, findings: &mut Vec<Finding>) {
    check_m7_foundation_contracts(root, findings);
    check_m7_example_contracts(root, findings);
    check_m7_inspection_contracts(root, findings);
    check_m7_visual_browser_contracts(root, findings);
}
