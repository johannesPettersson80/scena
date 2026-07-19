use super::{SceneHostError, SceneHostErrorCode};

// Host handles cross JSON and browser boundaries, so every valid value must
// remain exactly representable by an IEEE-754 number (<= 2^53 - 1).
const SLOT_BITS: u32 = 28;
const GENERATION_BITS: u32 = 21;
const GENERATION_SHIFT: u32 = SLOT_BITS;
const KIND_SHIFT: u32 = SLOT_BITS + GENERATION_BITS;
const SLOT_MASK: u64 = (1_u64 << SLOT_BITS) - 1;
const MAX_GENERATION: u32 = (1_u32 << GENERATION_BITS) - 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub(super) enum HandleKind {
    Node = 1,
    Import = 2,
    InstanceRoot = 3,
    Animation = 4,
}

impl HandleKind {
    const fn from_tag(tag: u64) -> Option<Self> {
        match tag {
            1 => Some(Self::Node),
            2 => Some(Self::Import),
            3 => Some(Self::InstanceRoot),
            4 => Some(Self::Animation),
            _ => None,
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::Node => "node",
            Self::Import => "import",
            Self::InstanceRoot => "instance_root",
            Self::Animation => "animation",
        }
    }
}

#[derive(Debug, Clone)]
pub(super) struct HandleTable<T> {
    slots: Vec<HandleSlot<T>>,
    kind: HandleKind,
}

#[derive(Debug, Clone)]
struct HandleSlot<T> {
    generation: u32,
    value: Option<T>,
    retired: bool,
}

impl<T> HandleTable<T> {
    pub(super) const fn new(kind: HandleKind) -> Self {
        Self {
            slots: Vec::new(),
            kind,
        }
    }

    pub(super) fn insert(&mut self, value: T) -> u64 {
        if let Some((index, slot)) = self
            .slots
            .iter_mut()
            .enumerate()
            .find(|(_, slot)| slot.value.is_none() && !slot.retired)
        {
            slot.value = Some(value);
            return encode_handle(self.kind, index, slot.generation);
        }

        self.slots.push(HandleSlot {
            generation: 1,
            value: Some(value),
            retired: false,
        });
        encode_handle(self.kind, self.slots.len() - 1, 1)
    }

    pub(super) fn get(
        &self,
        handle: u64,
        missing_code: SceneHostErrorCode,
        stale_code: SceneHostErrorCode,
    ) -> Result<&T, SceneHostError> {
        let decoded = decode_handle(handle).ok_or_else(|| {
            SceneHostError::new(
                missing_code,
                format!("host handle {handle} is outside this handle table"),
            )
        })?;
        self.ensure_kind(handle, decoded.kind)?;
        let (index, generation) = (decoded.index, decoded.generation);
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

    pub(super) fn get_mut(
        &mut self,
        handle: u64,
        missing_code: SceneHostErrorCode,
        stale_code: SceneHostErrorCode,
    ) -> Result<&mut T, SceneHostError> {
        let decoded = decode_handle(handle).ok_or_else(|| {
            SceneHostError::new(
                missing_code,
                format!("host handle {handle} is outside this handle table"),
            )
        })?;
        self.ensure_kind(handle, decoded.kind)?;
        let (index, generation) = (decoded.index, decoded.generation);
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
        slot.value.as_mut().ok_or_else(|| {
            SceneHostError::new(stale_code, format!("host handle {handle} is stale"))
        })
    }

    pub(super) fn remove(
        &mut self,
        handle: u64,
        missing_code: SceneHostErrorCode,
        stale_code: SceneHostErrorCode,
    ) -> Result<T, SceneHostError> {
        let decoded = decode_handle(handle).ok_or_else(|| {
            SceneHostError::new(
                missing_code,
                format!("host handle {handle} is outside this handle table"),
            )
        })?;
        self.ensure_kind(handle, decoded.kind)?;
        let (index, generation) = (decoded.index, decoded.generation);
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
        if slot.generation == MAX_GENERATION {
            slot.retired = true;
        } else {
            slot.generation += 1;
        }
        Ok(value)
    }

    pub(super) fn values(&self) -> impl Iterator<Item = &T> {
        self.slots.iter().filter_map(|slot| slot.value.as_ref())
    }

    pub(super) fn entries(&self) -> impl Iterator<Item = (u64, &T)> {
        self.slots.iter().enumerate().filter_map(|(index, slot)| {
            slot.value
                .as_ref()
                .map(|value| (encode_handle(self.kind, index, slot.generation), value))
        })
    }

    fn ensure_kind(&self, handle: u64, actual: HandleKind) -> Result<(), SceneHostError> {
        if actual == self.kind {
            return Ok(());
        }
        Err(SceneHostError::new(
            SceneHostErrorCode::WrongHandleNamespace,
            format!(
                "host handle {handle} belongs to the {} namespace, expected {}",
                actual.label(),
                self.kind.label()
            ),
        ))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DecodedHandle {
    kind: HandleKind,
    index: usize,
    generation: u32,
}

pub(super) fn handle_kind(handle: u64) -> Option<HandleKind> {
    decode_handle(handle).map(|decoded| decoded.kind)
}

fn encode_handle(kind: HandleKind, index: usize, generation: u32) -> u64 {
    let slot = index as u64 + 1;
    assert!(
        slot <= SLOT_MASK,
        "SceneHost handle table exhausted its slot field"
    );
    assert!(
        (1..=MAX_GENERATION).contains(&generation),
        "SceneHost handle generation is outside the encoded range"
    );
    (u64::from(kind as u8) << KIND_SHIFT) | (u64::from(generation) << GENERATION_SHIFT) | slot
}

fn decode_handle(handle: u64) -> Option<DecodedHandle> {
    if handle > ((1_u64 << 53) - 1) {
        return None;
    }
    let kind = HandleKind::from_tag(handle >> KIND_SHIFT)?;
    let generation = ((handle >> GENERATION_SHIFT) & u64::from(MAX_GENERATION)) as u32;
    let slot = handle & SLOT_MASK;
    if generation == 0 || slot == 0 {
        return None;
    }
    Some(DecodedHandle {
        kind,
        index: (slot - 1) as usize,
        generation,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const KINDS: [HandleKind; 4] = [
        HandleKind::Node,
        HandleKind::Import,
        HandleKind::InstanceRoot,
        HandleKind::Animation,
    ];

    const fn codes(kind: HandleKind) -> (SceneHostErrorCode, SceneHostErrorCode) {
        match kind {
            HandleKind::Node | HandleKind::InstanceRoot => (
                SceneHostErrorCode::NodeHandleNotFound,
                SceneHostErrorCode::StaleNodeHandle,
            ),
            HandleKind::Import => (
                SceneHostErrorCode::ImportHandleNotFound,
                SceneHostErrorCode::StaleImportHandle,
            ),
            HandleKind::Animation => (
                SceneHostErrorCode::AnimationHandleNotFound,
                SceneHostErrorCode::StaleAnimationHandle,
            ),
        }
    }

    #[test]
    fn every_live_handle_kind_is_rejected_by_every_other_table() {
        let handles = KINDS.map(|kind| {
            let mut table = HandleTable::new(kind);
            let handle = table.insert(kind as u8);
            (kind, handle)
        });

        for expected_kind in KINDS {
            let mut table = HandleTable::new(expected_kind);
            let expected_handle = table.insert(expected_kind as u8);
            let (missing, stale) = codes(expected_kind);
            for (actual_kind, handle) in handles {
                let result = table.get(handle, missing, stale);
                if actual_kind == expected_kind {
                    assert_eq!(handle, expected_handle);
                    assert_eq!(result.copied(), Ok(expected_kind as u8));
                } else {
                    assert_eq!(
                        result.expect_err("wrong namespace must fail").code(),
                        SceneHostErrorCode::WrongHandleNamespace,
                        "{actual_kind:?} handle passed to {expected_kind:?} table"
                    );
                    assert_eq!(
                        table
                            .get_mut(handle, missing, stale)
                            .expect_err("wrong namespace must fail before mutable access")
                            .code(),
                        SceneHostErrorCode::WrongHandleNamespace
                    );
                    assert_eq!(
                        table
                            .remove(handle, missing, stale)
                            .expect_err("wrong namespace must fail before removal")
                            .code(),
                        SceneHostErrorCode::WrongHandleNamespace
                    );
                    assert_eq!(
                        table.get(expected_handle, missing, stale).copied(),
                        Ok(expected_kind as u8),
                        "wrong-kind removal must not mutate the expected table"
                    );
                }
            }
        }
    }

    #[test]
    fn every_namespace_reuses_slots_with_a_new_generation_and_rejects_old_handles() {
        for kind in KINDS {
            let mut table = HandleTable::new(kind);
            let (missing, stale) = codes(kind);
            let first = table.insert(10_u8);
            assert_eq!(table.remove(first, missing, stale), Ok(10));
            assert_eq!(
                table
                    .get(first, missing, stale)
                    .expect_err("removed handle is stale")
                    .code(),
                stale
            );

            let reused = table.insert(20);
            assert_ne!(first, reused);
            assert_eq!(decode_handle(first).expect("first decodes").index, 0);
            assert_eq!(decode_handle(reused).expect("reused decodes").index, 0);
            assert_eq!(table.get(reused, missing, stale).copied(), Ok(20));
            assert_eq!(
                table
                    .get(first, missing, stale)
                    .expect_err("old generation stays stale")
                    .code(),
                stale
            );
        }
    }

    #[test]
    fn same_namespace_missing_slots_are_not_reported_as_stale() {
        for kind in KINDS {
            let table = HandleTable::<u8>::new(kind);
            let (missing, stale) = codes(kind);
            let never_allocated = encode_handle(kind, 99, MAX_GENERATION);
            let error = table
                .get(never_allocated, missing, stale)
                .expect_err("never-allocated slot is missing");
            assert_eq!(error.code(), missing);
        }
    }

    #[test]
    fn exhausted_high_generation_slots_retire_instead_of_repeating_a_handle() {
        for kind in KINDS {
            let mut table = HandleTable::new(kind);
            let (missing, stale) = codes(kind);
            table.slots.push(HandleSlot {
                generation: MAX_GENERATION - 1,
                value: Some(10_u8),
                retired: false,
            });
            let penultimate = encode_handle(kind, 0, MAX_GENERATION - 1);
            table
                .remove(penultimate, missing, stale)
                .expect("high generation removes");

            let last = table.insert(20);
            assert_eq!(
                decode_handle(last).expect("last decodes").generation,
                MAX_GENERATION
            );
            table
                .remove(last, missing, stale)
                .expect("last generation removes");
            assert_eq!(
                table
                    .get(last, missing, stale)
                    .expect_err("retired handle is stale")
                    .code(),
                stale
            );

            let next_slot = table.insert(30);
            let decoded = decode_handle(next_slot).expect("next slot decodes");
            assert_eq!(decoded.index, 1, "exhausted slot must never be reused");
            assert_eq!(decoded.generation, 1);
            assert_ne!(last, next_slot);
        }
    }

    #[test]
    fn all_encoded_kinds_fit_the_exact_javascript_integer_range() {
        for kind in KINDS {
            let maximum = encode_handle(kind, SLOT_MASK as usize - 1, MAX_GENERATION);
            assert!(maximum < (1_u64 << 53));
            assert_eq!(handle_kind(maximum), Some(kind));
        }
    }
}
