use scena::{Backend, Capabilities, CapabilityStatus};

#[test]
fn browser_backends_report_the_renderer_owned_sample_count_matrix() {
    for backend in [Backend::WebGpu, Backend::WebGl2] {
        let capabilities = Capabilities::for_attached_gpu_backend(backend);
        let value = serde_json::to_value(capabilities).expect("capabilities serialize");
        assert_eq!(value["render_sample_counts"], serde_json::json!([1, 0, 0]));
        assert_eq!(value["depth_sample_counts"], serde_json::json!([1, 0, 0]));
        assert_eq!(
            value["explicit_msaa"],
            serde_json::json!(CapabilityStatus::ErrorIfRequired)
        );
        assert_eq!(value["color_target_format"], "Rgba8UnormSrgb");
    }
}

#[test]
fn capability_reports_accept_the_pre_sample_matrix_shape() {
    let mut value = serde_json::to_value(Capabilities::for_backend(Backend::Headless))
        .expect("capabilities serialize");
    let object = value.as_object_mut().expect("capabilities are an object");
    object.remove("render_sample_counts");
    object.remove("depth_sample_counts");
    object.remove("explicit_msaa");

    let decoded: Capabilities =
        serde_json::from_value(value).expect("pre-sample-matrix capabilities deserialize");
    assert_eq!(decoded.render_sample_counts, [1, 0, 0]);
    assert_eq!(decoded.depth_sample_counts, [1, 0, 0]);
    assert_eq!(decoded.explicit_msaa, CapabilityStatus::ErrorIfRequired);
}
