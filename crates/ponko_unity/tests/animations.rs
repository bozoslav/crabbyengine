use ponko_unity::{AnimationClip, MeshSkin, SkinNode};

#[test]
fn relative_paths_walk_from_explicit_root_and_preserve_rest_components() {
    let clip = AnimationClip::parse("--- !u!74 &1\nAnimationClip:\n  m_Name: Reach\n  m_SampleRate: 2\n  m_AnimationClipSettings: {m_StartTime: 1, m_StopTime: 2}\n  m_PositionCurves:\n  - path: Arm/Hand\n    curve:\n      m_Curve:\n      - {time: 1, value: {x: 0}, outSlope: {x: 4}}\n      - {time: 2, value: {x: 2}, inSlope: {x: 0}}\n").unwrap();
    let mut skin = MeshSkin {
        nodes: ["Root", "Arm", "Hand"]
            .into_iter()
            .enumerate()
            .map(|(i, name)| SkinNode {
                name: name.into(),
                parent: i.checked_sub(1),
                matrix: glam::Mat4::from_scale(glam::Vec3::splat(2.0)).to_cols_array(),
            })
            .collect(),
        ..Default::default()
    };
    assert!(clip.append_to_skin(&mut skin, 0).is_empty());
    let animation = &skin.animations[0];
    assert!(animation.nodes[0].is_empty());
    assert!(animation.nodes[1].is_empty());
    assert_eq!(animation.times, [0.0, 0.5, 1.0]);
    let (scale, _, position) = glam::Mat4::from_cols_array(&animation.nodes[2][1]).to_scale_rotation_translation();
    assert_eq!(position.x, 1.5);
    assert_eq!(scale, glam::Vec3::splat(2.0));
}
