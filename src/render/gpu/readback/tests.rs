use super::select_completed_meter_slot;

#[test]
fn newest_completed_meter_sample_wins_regardless_of_slot_index() {
    // Slot 1 carries the newer submission. Consuming slot 0 first would
    // apply a stale frame's exposure after a newer one was already
    // available, which reads as exposure stepping backward.
    assert_eq!(
        select_completed_meter_slot(&[Some(5), Some(7)], None),
        Some(1),
    );
    assert_eq!(
        select_completed_meter_slot(&[Some(7), Some(5)], None),
        Some(0),
    );
}

#[test]
fn meter_samples_older_than_the_last_applied_sequence_are_rejected() {
    // A slot that completes late must never be applied once a newer
    // sample has already driven exposure.
    assert_eq!(select_completed_meter_slot(&[Some(3), None], Some(6)), None);
    assert_eq!(
        select_completed_meter_slot(&[Some(3), Some(9)], Some(6)),
        Some(1),
    );
    assert_eq!(select_completed_meter_slot(&[None, None], Some(6)), None);
}
