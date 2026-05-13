#![allow(unused_imports)]

mod architecture_map;
mod core;
mod doctor_architecture;
mod doctor_core;
mod doctor_docs;
mod doctor_m7_m8_assets;
mod doctor_render;
mod doctor_scene_platform;
mod doctor_visual_release;
mod prelude;
mod release;
#[cfg(test)]
mod tests_01;
#[cfg(test)]
mod tests_02;
#[cfg(test)]
mod tests_03;
#[cfg(test)]
mod tests_04;
#[cfg(test)]
mod tests_05;
#[cfg(test)]
mod tests_06;
#[cfg(test)]
mod tests_07;
#[cfg(test)]
mod tests_08;
#[cfg(test)]
mod tests_09;
#[cfg(test)]
mod tests_10;
mod util;
mod visual_artifacts;
mod visual_proof;

pub(crate) use core::run;
