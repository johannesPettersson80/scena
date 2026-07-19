use super::*;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct CpuRowBandMetrics {
    pub(super) workers: u64,
    pub(super) candidate_triangles: u64,
    pub(super) full_rescan_triangles: u64,
    pub(super) storage_growth_bytes: u64,
}

#[derive(Debug, Default)]
pub(in crate::render) struct CpuRowBandBins {
    pub(super) bands: Vec<Vec<usize>>,
    pub(super) projected_bounds: Vec<Option<(u32, u32)>>,
    pub(super) rows_per_band: usize,
}

impl CpuRowBandBins {
    pub(super) fn rebuild(
        &mut self,
        primitives: &[PreparedPrimitive],
        target: RasterTarget,
        camera: &camera::CameraProjection,
        requested_workers: usize,
    ) -> CpuRowBandMetrics {
        let workers = requested_workers
            .max(1)
            .min(super::super::parallel::worker_count(primitives.len()));
        let band_count = workers.min(target.height as usize).max(1);
        self.rows_per_band = (target.height as usize).div_ceil(band_count).max(1);
        let before_bytes = self.storage_capacity_bytes();
        self.bands.resize_with(band_count, Vec::new);
        for band in &mut self.bands {
            band.clear();
        }
        self.projected_bounds.resize(primitives.len(), None);

        #[cfg(not(target_arch = "wasm32"))]
        if workers > 1 {
            self.projected_bounds
                .par_iter_mut()
                .zip(primitives.par_iter())
                .for_each(|(bounds, primitive)| {
                    *bounds = cpu::primitive_screen_row_bounds(primitive, target, camera);
                });
        } else {
            for (bounds, primitive) in self.projected_bounds.iter_mut().zip(primitives) {
                *bounds = cpu::primitive_screen_row_bounds(primitive, target, camera);
            }
        }
        #[cfg(target_arch = "wasm32")]
        for (bounds, primitive) in self.projected_bounds.iter_mut().zip(primitives) {
            *bounds = cpu::primitive_screen_row_bounds(primitive, target, camera);
        }

        for (primitive_index, bounds) in self.projected_bounds.iter().copied().enumerate() {
            let Some((min_row, max_row)) = bounds else {
                continue;
            };
            let first_band = min_row as usize / self.rows_per_band;
            let last_band = (max_row as usize / self.rows_per_band).min(band_count - 1);
            for band in &mut self.bands[first_band..=last_band] {
                band.push(primitive_index);
            }
        }
        let after_bytes = self.storage_capacity_bytes();
        CpuRowBandMetrics {
            workers: workers as u64,
            candidate_triangles: self.bands.iter().map(|band| band.len() as u64).sum(),
            full_rescan_triangles: (primitives.len() as u64).saturating_mul(band_count as u64),
            storage_growth_bytes: after_bytes.saturating_sub(before_bytes) as u64,
        }
    }

    fn storage_capacity_bytes(&self) -> usize {
        self.bands
            .capacity()
            .saturating_mul(std::mem::size_of::<Vec<usize>>())
            .saturating_add(
                self.projected_bounds
                    .capacity()
                    .saturating_mul(std::mem::size_of::<Option<(u32, u32)>>()),
            )
            .saturating_add(
                self.bands
                    .iter()
                    .map(|band| band.capacity().saturating_mul(std::mem::size_of::<usize>()))
                    .sum::<usize>(),
            )
    }

    #[cfg(test)]
    pub(super) fn band_count(&self) -> usize {
        self.bands.len()
    }

    #[cfg(test)]
    pub(super) fn bands(&self) -> &[Vec<usize>] {
        &self.bands
    }

    #[cfg(test)]
    pub(super) fn capacities(&self) -> Vec<usize> {
        self.bands.iter().map(Vec::capacity).collect()
    }
}

pub(super) fn selected_primitives<'a>(
    primitives: &'a [PreparedPrimitive],
    indices: Option<&'a [usize]>,
) -> impl Iterator<Item = &'a PreparedPrimitive> {
    let full = primitives
        .iter()
        .take(if indices.is_none() { usize::MAX } else { 0 });
    let selected = indices
        .into_iter()
        .flat_map(move |indices| indices.iter().map(move |&index| &primitives[index]));
    full.chain(selected)
}

pub(super) fn resize_reusable_scratch<T: Clone>(scratch: &mut Vec<T>, len: usize, value: T) -> u64 {
    let previous_capacity = scratch.capacity();
    scratch.resize(len, value);
    (scratch.capacity().saturating_sub(previous_capacity) as u64)
        .saturating_mul(std::mem::size_of::<T>() as u64)
}
