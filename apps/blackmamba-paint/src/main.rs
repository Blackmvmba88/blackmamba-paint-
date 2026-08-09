use bmp_ai::{AnalysisRequest, InsightKind, MockVisualIntelligence, VisualIntelligence};
use bmp_core::{Document, Layer, PointSample, Stroke};
use bmp_perspective::{LineObservation, PerspectiveInference, StubPerspectiveInference};

fn main() {
    let mut document = Document::new("BlackMamba Paint — First AI-Native Stroke");
    document.ai.artist_intent = Some("cinematic industrial concept art".into());
    document.ai.intent_locks.preserve = vec!["silhouette".into(), "gesture".into()];

    let layer = Layer::new("ink");
    let layer_id = document.add_layer(layer);

    let stroke = Stroke::new(
        layer_id,
        "graphite",
        vec![
            PointSample {
                x: 0.0,
                y: 0.0,
                pressure: 0.4,
                tilt_x: 0.0,
                tilt_y: 0.0,
                timestamp_ms: 0,
            },
            PointSample {
                x: 180.0,
                y: 64.0,
                pressure: 0.9,
                tilt_x: 0.1,
                tilt_y: 0.0,
                timestamp_ms: 420,
            },
        ],
    );

    document.add_stroke(stroke).expect("stroke should attach to layer");

    let ai = MockVisualIntelligence;
    let analysis = ai.analyze(
        &document,
        &AnalysisRequest {
            goal: "find the highest-impact improvement without changing intent".into(),
            enabled_kinds: vec![
                InsightKind::Perspective,
                InsightKind::Composition,
                InsightKind::Edge,
                InsightKind::Learning,
            ],
        },
    );

    let perspective = StubPerspectiveInference.infer(
        &[LineObservation {
            x1: 0.0,
            y1: 0.0,
            x2: 180.0,
            y2: 64.0,
            weight: 1.0,
        }],
        document.projection,
    );

    println!("BlackMamba Paint foundation online");
    println!("strokes: {}", document.strokes.len());
    println!("ai insights: {}", analysis.insights.len());
    println!("perspective confidence: {:.2}", perspective.confidence);
    println!("preserve locks: {:?}", analysis.critique.do_not_touch);
}
