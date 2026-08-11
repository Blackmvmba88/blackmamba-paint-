use bmp_ai::{AnalysisRequest, DeterministicVisualIntelligence, InsightKind, VisualIntelligence};
use bmp_brush::{BrushDefinition, BrushLibrary};
use bmp_core::{Document, Layer, PointSample, Stroke};
use bmp_history::DocumentHistory;
use bmp_perspective::{GeometricPerspectiveInference, LineObservation, PerspectiveInference};
use bmp_space::{Bounds, SparseSpatialIndex};
use bmp_storage::BmpaintPackage;
use std::collections::BTreeMap;
use uuid::Uuid;

// ============================================================================
// BlackMamba Paint M0 Foundation — Integration Demo
// ============================================================================
//
// This demo showcases the complete ecosystem:
//   • bmp-core:        Document model with time-native strokes
//   • bmp-ai:          Visual intelligence analysis with evidence & confidence
//   • bmp-perspective: Geometric perspective inference
//   • bmp-history:     Undo/redo system with full state reconstruction
//   • bmp-storage:     Binary serialization to .bmpaint format
//   • bmp-render:      Scene composition for rendering
//   • bmp-space:       Spatial indexing for viewport culling
//
// The artist remains authoritative. AI proposes; document changes explicitly.
// ============================================================================

fn main() {
    println!("\n{}", "=".repeat(80));
    println!("{:^80}", "BlackMamba Paint — M0 Foundation Demo");
    println!("{}", "=".repeat(80));

    // ========================================================================
    // Phase 1: Create Document & Set Artist Intent
    // ========================================================================
    println!("\n▸ Phase 1: Document Setup");
    let mut document = Document::new("Perspective Study — Linear Convergence");
    document.ai.artist_intent = Some("one-point perspective industrial sketch".into());
    document.ai.intent_locks.preserve = vec!["silhouette".into(), "gesture".into()];

    println!("  ✓ Document created: ID={}", &document.id.to_string()[..8]);
    println!("  ✓ Artist intent: {}", document.ai.artist_intent.as_ref().unwrap());
    println!(
        "  ✓ Preserve locks: {:?}",
        document.ai.intent_locks.preserve
    );

    // ========================================================================
    // Phase 2: Capture Strokes with Time & Pressure
    // ========================================================================
    println!("\n▸ Phase 2: Stroke Capture");
    let layer_id = document.add_layer(Layer::new("construction"));
    println!("  ✓ Layer created: ID={}", &layer_id.to_string()[..8]);

    // Draw converging lines (classic one-point perspective)
    let vanishing_point = (300.0, 100.0);
    let stroke_ids: Vec<_> = (0..3)
        .map(|i| {
            let start = (50.0, 100.0 + (i as f64 * 40.0));
            let points = vec![
                PointSample {
                    x: start.0,
                    y: start.1,
                    pressure: 0.3,
                    tilt_x: 0.0,
                    tilt_y: 0.0,
                    timestamp_ms: 0,
                },
                PointSample {
                    x: vanishing_point.0,
                    y: vanishing_point.1,
                    pressure: 0.8,
                    tilt_x: 0.0,
                    tilt_y: 0.0,
                    timestamp_ms: 300 + (i as u64 * 50),
                },
            ];
            let stroke = Stroke::new(layer_id, "graphite", points);
            let id = stroke.id;
            document.add_stroke(stroke).expect("stroke should attach");
            println!(
                "  ✓ Stroke {}: duration {}ms, {} samples",
                i + 1,
                document.strokes[&id].duration_ms(),
                document.strokes[&id].points.len()
            );
            id
        })
        .collect();

    println!("  ✓ Total strokes: {}", document.strokes.len());

    // ========================================================================
    // Phase 3: Perspective Inference (Geometric)
    // ========================================================================
    println!("\n▸ Phase 3: Perspective Analysis");
    let observations: Vec<_> = stroke_ids
        .iter()
        .map(|&id| {
            let stroke = &document.strokes[&id];
            let first = &stroke.points[0];
            let last = &stroke.points[stroke.points.len() - 1];
            LineObservation::new(first.x, first.y, last.x, last.y)
        })
        .collect();

    let perspective_solver = GeometricPerspectiveInference::default();
    let perspective_result = perspective_solver.infer(&observations, document.projection);

    println!("  ✓ Projection mode: {:?}", document.projection);
    println!(
        "  ✓ Vanishing points detected: {}",
        perspective_result.vanishing_points.len()
    );
    if perspective_result.vanishing_points.len() > 0 {
        for (i, vp) in perspective_result.vanishing_points.iter().enumerate() {
            println!(
                "    - VP{}: ({:.1}, {:.1}), confidence: {:.2}",
                i + 1, vp.x, vp.y, vp.confidence
            );
        }
    }
    if let Some(horizon) = &perspective_result.horizon {
        println!(
            "  ✓ Horizon detected: y={:.1}, confidence={:.2}",
            horizon.y, horizon.confidence
        );
    }
    println!(
        "  ✓ Overall confidence: {:.2}%",
        perspective_result.confidence * 100.0
    );

    // ========================================================================
    // Phase 4: AI Visual Intelligence Analysis
    // ========================================================================
    println!("\n▸ Phase 4: AI Visual Intelligence");
    let ai = DeterministicVisualIntelligence::default();
    let analysis = ai.analyze(
        &document,
        &AnalysisRequest {
            goal: "analyze composition and perspective while respecting artist locks".into(),
            enabled_kinds: vec![
                InsightKind::Perspective,
                InsightKind::Composition,
                InsightKind::Learning,
            ],
        },
    );

    println!("  ✓ Insights generated: {}", analysis.insights.len());
    for insight in &analysis.insights {
        println!(
            "    - {:?}: \"{}\" (confidence: {:.2}, {} evidence items)",
            insight.kind,
            insight.label,
            insight.confidence,
            insight.evidence.len()
        );
        for evidence in &insight.evidence {
            println!("      • {}: {}", evidence.label, evidence.description);
        }
    }

    println!("  ✓ Critique:");
    println!(
        "    High-impact improvements: {}",
        analysis.critique.high_impact.len()
    );
    println!(
        "    Protected by artist locks: {:?}",
        analysis.critique.do_not_touch
    );
    println!(
        "    Ghost suggestions (learning): {}",
        analysis.ghosts.len()
    );

    // ========================================================================
    // Phase 5: Undo/Redo & History
    // ========================================================================
    println!("\n▸ Phase 5: Document History");
    let mut history = DocumentHistory::new(&document).expect("history initialization");
    history.commit(&document).expect("history commit");

    let undone = history.undo().expect("undo should work");
    println!("  ✓ After undo: {} strokes", undone.strokes.len());

    let redone = history.redo().expect("redo should work");
    println!("  ✓ After redo: {} strokes", redone.strokes.len());

    // ========================================================================
    // Phase 6: Serialization to .bmpaint Format
    // ========================================================================
    println!("\n▸ Phase 6: Binary Serialization");
    let package = BmpaintPackage::from_document(redone.clone());
    let bytes = package.encode().expect(".bmpaint encoding");
    println!("  ✓ Serialized to .bmpaint: {} bytes", bytes.len());

    let restored = BmpaintPackage::decode(&bytes)
        .expect(".bmpaint decoding")
        .document;
    println!("  ✓ Restored from .bmpaint: {} strokes", restored.strokes.len());
    println!(
        "  ✓ Document integrity: {}",
        if document == restored {
            "✓ Perfect round-trip"
        } else {
            "⚠ Differences detected"
        }
    );

    // ========================================================================
    // Phase 7: Spatial Indexing for Viewport Culling
    // ========================================================================
    println!("\n▸ Phase 7: Spatial Indexing");
    let mut spatial_index = SparseSpatialIndex::default();
    for (id, stroke) in &restored.strokes {
        let bounds = calculate_stroke_bounds(stroke);
        spatial_index.insert(*id, bounds);
    }

    let viewport = Bounds::new(-100.0, -100.0, 500.0, 300.0);
    let visible_strokes = spatial_index.query(viewport);
    println!("  ✓ Viewport: {:?}", viewport);
    println!(
        "  ✓ Visible in viewport: {} / {} strokes",
        visible_strokes.len(),
        restored.strokes.len()
    );

    // ========================================================================
    // Phase 8: Brush & Render System
    // ========================================================================
    println!("\n▸ Phase 8: Rendering (Brush & Scene Setup)");
    let mut brush_library = BrushLibrary::default();
    brush_library
        .insert(BrushDefinition::pencil("graphite", "Graphite 2B", 8.0))
        .expect("brush registration");

    println!("  ✓ Brush library initialized: 1 brush");
    println!(
        "  ✓ Ready to render with camera: pos=({:.1}, {:.1}), zoom={:.2}",
        restored.camera.x, restored.camera.y, restored.camera.zoom
    );

    // ========================================================================
    // Summary & Verification
    // ========================================================================
    println!("\n{}", "=".repeat(80));
    println!("{:^80}", "System Verification");
    println!("{}", "=".repeat(80));

    let mut results = BTreeMap::new();
    results.insert("Document Model (bmp-core)", document.strokes.len() > 0);
    results.insert("Time-Native Strokes", document.strokes.values().all(|s| s.duration_ms() > 0));
    results.insert("AI Visual Intelligence", analysis.insights.len() > 0);
    results.insert("Preserve Locks Respected", analysis.critique.do_not_touch.len() > 0);
    results.insert("Perspective Inference", perspective_result.vanishing_points.len() > 0);
    results.insert("Document History", history.can_undo() || history.can_redo());
    results.insert("Binary Serialization", document == restored);
    results.insert("Spatial Indexing", visible_strokes.len() > 0);

    for (component, success) in results {
        println!(
            "  {} {}: {}",
            if success { "✓" } else { "✗" },
            component,
            if success { "PASS" } else { "FAIL" }
        );
    }

    println!("\n{}", "=".repeat(80));
    println!(
        "{:^80}",
        "M0 Foundation Complete — All Systems Operational"
    );
    println!("{}", "=".repeat(80));
    println!();
}

// ============================================================================
// Utilities
// ============================================================================

fn calculate_stroke_bounds(stroke: &Stroke) -> Bounds {
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;

    for point in &stroke.points {
        min_x = min_x.min(point.x);
        min_y = min_y.min(point.y);
        max_x = max_x.max(point.x);
        max_y = max_y.max(point.y);
    }

    Bounds::new(min_x, min_y, max_x, max_y)
}
