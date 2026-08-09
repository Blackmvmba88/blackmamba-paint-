use bmp_animation::{build_reveal_plan, RevealBranch, RevealDirection, RevealTiming};
use bmp_trace::{trace_line, PixelCoord, RasterImage, Rgba, TraceConfig};

#[test]
fn clicked_tracepulse_seed_becomes_animation_time_zero() {
    let mut image = RasterImage::filled(9, 5, Rgba::WHITE).unwrap();
    for x in 1..=5 {
        image.set_pixel(PixelCoord::new(x, 2), Rgba::BLACK).unwrap();
    }

    let trace = trace_line(
        &image,
        PixelCoord::new(3, 2),
        TraceConfig {
            channel_tolerance: 0,
            ..TraceConfig::default()
        },
    )
    .unwrap();

    let plan = build_reveal_plan(
        &trace.path,
        trace.seed_node_index,
        RevealDirection::Both,
        RevealTiming::FixedDuration { duration_ms: 800 },
    )
    .unwrap();

    assert!(plan.events.iter().any(|event| {
        event.node_index == trace.seed_node_index
            && event.at_ms == 0
            && event.branch == RevealBranch::Seed
    }));
    assert_eq!(plan.duration_ms, 800);
}
