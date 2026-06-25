use super::*;

#[test]
fn matching_profile_sidecar_defers_hdr_decode_until_fallback_needed() {
    let path = "memory://uniform-studio.hdr";
    let bytes = rle_radiance_hdr_uniform(8, 1, [64, 32, 16, 129]);
    let sidecar = minimal_sidecar(&bytes);

    let desc = EnvironmentDesc::from_equirectangular_hdr_sidecar_bytes(path, &bytes, sidecar)
        .expect("sidecar env constructs")
        .expect("sidecar sha matches source");

    assert!(
        desc.prefilter_sidecar(EnvironmentSidecarProfile::InteractiveWebGl2)
            .is_some(),
        "precondition: the WebGL2 sidecar must satisfy a matching WebGL2 request"
    );
    assert!(
        desc.equirectangular_pixels.is_none(),
        "matching-profile sidecar loads must not decode and retain the HDR pixel grid; \
         decode is only allowed when a later profile-mismatch fallback needs a fresh bake"
    );
    assert!(
        desc.lazy_equirectangular_source.is_some(),
        "sidecar-backed environments must retain compressed HDR source bytes for a later \
         profile-mismatch fallback"
    );
}

fn minimal_sidecar(source_bytes: &[u8]) -> EnvironmentPrefilterSidecar {
    let face = vec![0.5, 0.5, 0.5, 1.0];
    let mip = [
        face.clone(),
        face.clone(),
        face.clone(),
        face.clone(),
        face.clone(),
        face,
    ];
    EnvironmentPrefilterSidecar::new(
        EnvironmentSidecarProfile::InteractiveWebGl2,
        &sha256_hex(source_bytes),
        1,
        vec![mip],
        vec![1.0, 0.0, 1.0, 0.0],
        1,
        [0.5, 0.5, 0.5],
    )
    .expect("minimal sidecar is valid")
}

fn rle_radiance_hdr_uniform(width: u32, height: u32, rgbe: [u8; 4]) -> Vec<u8> {
    assert!(width >= 8);
    assert!(width <= 127);
    let mut bytes =
        format!("#?RADIANCE\nFORMAT=32-bit_rle_rgbe\n\n-Y {height} +X {width}\n").into_bytes();
    for _ in 0..height {
        bytes.push(0x02);
        bytes.push(0x02);
        bytes.push((width >> 8) as u8);
        bytes.push((width & 0xff) as u8);
        for channel in &rgbe {
            bytes.push(0x80 + width as u8);
            bytes.push(*channel);
        }
    }
    bytes
}
