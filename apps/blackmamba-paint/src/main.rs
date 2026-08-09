use bmp_ai::{AnalysisRequest, DeterministicVisualIntelligence, InsightKind, VisualIntelligence};
use bmp_brush::{BrushDefinition, BrushLibrary};
use bmp_core::{Document, Layer};
use bmp_history::DocumentHistory;
use bmp_input::{PointerKind, PointerSample, StrokeCapture};
use bmp_perspective::{LineObservation, PerspectiveInference, StubPerspectiveInference};
use bmp_render::{build_render_scene, ScreenViewport};
use bmp_space::{Bounds, SparseSpatialIndex};
use bmp_storage::BmpaintPackage;

fn main() {
    let mut document = Document::new("BlackMamba Paint — First Stroke");
    document.ai.artist_intent = Some("cinematic industrial concept art".into());
    document.ai.intent_locks.preserve = vec!["silhouette".into(), "gesture".into()];

    let layer_id = document.add_layer(Layer::new("ink"));
    let mut history = DocumentHistory::new(&document).expect("history should initialize");

    let mut capture = StrokeCapture::begin(layer_id, "graphite");
    capture.push(PointerSample {
        kind: PointerKind::Pen,
        x: 0.0,
        y: 0.0,
        pressure: 0.4,
        tilt_x: 0.0,
        tilt_y: 0.0,
        timestamp_ms: 0,
    });
    capture.push(PointerSample {
        kind: PointerKind::Pen,
        x: 180.0,
        y: 64.0,
        pressure: 0.9,
        tilt_x: 0.1,
        tilt_y: 0.0,
        timestamp_ms: 420,
    });

    let stroke = capture.finish();
    let stroke_id = stroke.id;
    document
        .add_stroke(stroke)
        .expect("stroke should attach to layer");
    history.commit(&document).expect("history should commit");

    let mut spatial = SparseSpatialIndex::default();
    spatial.insert(stroke_id, Bounds::new(0.0, 0.0, 180.0, 64.0));
    let visible = spatial.query(Bounds::new(-10.0, -10.0, 200.0, 100.0));

    let undone = history.undo().expect("stroke should undo");
    let redone = history.redo().expect("stroke should redo");

    let package = BmpaintPackage::from_document(redone.clone());
    let bytes = package.encode().expect(".bmpaint should encode");
    let restored = BmpaintPackage::decode(&bytes)
        .expect(".bmpaint should decode")
        .document;

    let mut brushes = BrushLibrary::default();
    brushes
        .insert(BrushDefinition::pencil("graphite", "Graphite", 12.0))
        .expect("brush should register");
    let render_scene = build_render_scene(
        &restored,
        restored.camera,
        ScreenViewport::new(1280.0, 720.0).expect("viewport should be valid"),
        &brushes,
    )
    .expect("render scene should build");

    let ai = DeterministicVisualIntelligence::default();
    let analysis = ai.analyze(
        &restored,
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
        restored.projection,
    );

    println!("BlackMamba Paint pipeline online");
    println!("visible objects: {}", visible.len());
    println!("strokes after undo: {}", undone.strokes.len());
    println!("strokes after redo/load: {}", restored.strokes.len());
    println!("render dabs: {}", render_scene.dabs.len());
    println!(".bmpaint bytes: {}", bytes.len());
    println!("ai insights: {}", analysis.insights.len());
    println!("perspective confidence: {:.2}", perspective.confidence);
    println!("preserve locks: {:?}", analysis.critique.do_not_touch);
}
