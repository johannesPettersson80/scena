use super::{SceneHostError, SceneHostErrorCode};

const HANDLE_STRIDE: u64 = 1_u64 << 32;

#[derive(Debug, Clone)]
pub(super) struct HandleTable<T> {
    slots: Vec<HandleSlot<T>>,
}

#[derive(Debug, Clone)]
struct HandleSlot<T> {
    generation: u32,
    value: Option<T>,
}

impl<T> HandleTable<T> {
    pub(super) const fn new() -> Self {
        Self { slots: Vec::new() }
    }

    pub(super) fn insert(&mut self, value: T) -> u64 {
        if let Some((index, slot)) = self
            .slots
            .iter_mut()
            .enumerate()
            .find(|(_, slot)| slot.value.is_none())
        {
            slot.value = Some(value);
            return encode_handle(index, slot.generation);
        }

        self.slots.push(HandleSlot {
            generation: 1,
            value: Some(value),
        });
        encode_handle(self.slots.len() - 1, 1)
    }

    pub(super) fn get(
        &self,
        handle: u64,
        missing_code: SceneHostErrorCode,
        stale_code: SceneHostErrorCode,
    ) -> Result<&T, SceneHostError> {
        let (index, generation) = decode_handle(handle).ok_or_else(|| {
            SceneHostError::new(
                missing_code,
                format!("host handle {handle} is outside this handle table"),
            )
        })?;
        let Some(slot) = self.slots.get(index) else {
            return Err(SceneHostError::new(
                missing_code,
                format!("host handle {handle} is outside this handle table"),
            ));
        };
        if slot.generation != generation {
            return Err(SceneHostError::new(
                stale_code,
                format!("host handle {handle} is stale"),
            ));
        }
        slot.value.as_ref().ok_or_else(|| {
            SceneHostError::new(stale_code, format!("host handle {handle} is stale"))
        })
    }

    pub(super) fn remove(
        &mut self,
        handle: u64,
        missing_code: SceneHostErrorCode,
        stale_code: SceneHostErrorCode,
    ) -> Result<T, SceneHostError> {
        let (index, generation) = decode_handle(handle).ok_or_else(|| {
            SceneHostError::new(
                missing_code,
                format!("host handle {handle} is outside this handle table"),
            )
        })?;
        let Some(slot) = self.slots.get_mut(index) else {
            return Err(SceneHostError::new(
                missing_code,
                format!("host handle {handle} is outside this handle table"),
            ));
        };
        if slot.generation != generation {
            return Err(SceneHostError::new(
                stale_code,
                format!("host handle {handle} is stale"),
            ));
        }
        let value = slot.value.take().ok_or_else(|| {
            SceneHostError::new(stale_code, format!("host handle {handle} is stale"))
        })?;
        slot.generation = slot.generation.saturating_add(1).max(1);
        Ok(value)
    }
}

fn encode_handle(index: usize, generation: u32) -> u64 {
    u64::from(generation) * HANDLE_STRIDE + (index as u64 + 1)
}

fn decode_handle(handle: u64) -> Option<(usize, u32)> {
    let generation = handle / HANDLE_STRIDE;
    let slot = handle % HANDLE_STRIDE;
    if generation == 0 || slot == 0 || generation > u64::from(u32::MAX) {
        return None;
    }
    Some(((slot - 1) as usize, generation as u32))
}
