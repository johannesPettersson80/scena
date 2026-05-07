use crate::geometry::{Primitive, Vertex};
use crate::material::Color;
use crate::scene::{LabelBillboard, LabelDesc, Scene, Transform, Vec3};

pub(super) fn append_label_primitives(
    scene: &Scene,
    origin_shift: Vec3,
    primitives: &mut Vec<Primitive>,
) {
    for (_node, _label, label, transform) in scene.label_nodes() {
        append_label_billboard(label, transform, origin_shift, primitives);
    }
}

fn append_label_billboard(
    label: &LabelDesc,
    transform: Transform,
    origin_shift: Vec3,
    primitives: &mut Vec<Primitive>,
) {
    match label.billboard() {
        LabelBillboard::ScreenAligned => {
            let center = Vec3::new(
                transform.translation.x - origin_shift.x,
                transform.translation.y - origin_shift.y,
                transform.translation.z - origin_shift.z,
            );
            let half_width = label.size() * label.text().len().max(1) as f32 * 0.15;
            let half_height = label.size() * 0.25;
            let color = label.color();
            let z = center.z;
            let min = Vec3::new(center.x - half_width, center.y - half_height, z);
            let max = Vec3::new(center.x + half_width, center.y + half_height, z);
            push_quad(primitives, min, max, color);
        }
    }
}

fn push_quad(primitives: &mut Vec<Primitive>, min: Vec3, max: Vec3, color: Color) {
    let bottom_left = Vertex {
        position: Vec3::new(min.x, min.y, min.z),
        color,
    };
    let bottom_right = Vertex {
        position: Vec3::new(max.x, min.y, min.z),
        color,
    };
    let top_right = Vertex {
        position: Vec3::new(max.x, max.y, min.z),
        color,
    };
    let top_left = Vertex {
        position: Vec3::new(min.x, max.y, min.z),
        color,
    };
    primitives.push(Primitive::triangle([bottom_left, bottom_right, top_right]));
    primitives.push(Primitive::triangle([bottom_left, top_right, top_left]));
}
