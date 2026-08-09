use bmp_core::Document;
use bmp_perspective::{
    GeometricPerspectiveInference, LineObservation, PerspectiveInference, PerspectiveSolution,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum InsightKind {
    Perspective,
    Composition,
    Proportion,
    Anatomy,
    Value,
    Color,
    Edge,
    Material,
    Style,
    Context,
    Fashion,
    Learning,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RegionHint {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Evidence {
    pub label: String,
    pub confidence: f32,
    pub region: Option<RegionHint>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VisualInsight {
    pub id: Uuid,
    pub kind: InsightKind,
    pub claim: String,
    pub reason: String,
    pub suggested_action: Option<String>,
    pub confidence: f32,
    pub evidence: Vec<Evidence>,
    pub preserve: Vec<String>,
}

impl VisualInsight {
    pub fn new(kind: InsightKind, claim: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            kind,
            claim: claim.into(),
            reason: reason.into(),
            suggested_action: None,
            confidence: 0.0,
            evidence: Vec::new(),
            preserve: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GhostSuggestion {
    pub id: Uuid,
    pub label: String,
    pub explanation: String,
    pub confidence: f32,
    pub source_insight_ids: Vec<Uuid>,
    /// Geometry is intentionally abstract at this layer. The renderer/path crate
    /// will own concrete editable geometry so model providers cannot mutate pixels.
    pub geometry_hint: Vec<(f64, f64)>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Critique {
    pub high_impact: Vec<VisualInsight>,
    pub medium_impact: Vec<VisualInsight>,
    pub low_impact: Vec<VisualInsight>,
    pub do_not_touch: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnalysisRequest {
    pub goal: String,
    pub enabled_kinds: Vec<InsightKind>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AnalysisResult {
    pub insights: Vec<VisualInsight>,
    pub ghosts: Vec<GhostSuggestion>,
    pub critique: Critique,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WhyResponse {
    pub claim: String,
    pub because: String,
    pub confidence: f32,
    pub evidence: Vec<Evidence>,
    pub suggested_action: Option<String>,
    pub preserve: Vec<String>,
}

pub fn explain_why(insight: &VisualInsight) -> WhyResponse {
    WhyResponse {
        claim: insight.claim.clone(),
        because: insight.reason.clone(),
        confidence: insight.confidence,
        evidence: insight.evidence.clone(),
        suggested_action: insight.suggested_action.clone(),
        preserve: insight.preserve.clone(),
    }
}

/// Provider boundary for local models, cloud models, deterministic analyzers,
/// or future multimodal agents. The provider receives document state and returns
/// proposals. It never receives a mutable Document.
pub trait VisualIntelligence {
    fn analyze(&self, document: &Document, request: &AnalysisRequest) -> AnalysisResult;
}

/// Deterministic development engine retained for integration smoke tests.
#[derive(Debug, Default)]
pub struct MockVisualIntelligence;

impl VisualIntelligence for MockVisualIntelligence {
    fn analyze(&self, document: &Document, request: &AnalysisRequest) -> AnalysisResult {
        let mut insight = VisualInsight::new(
            InsightKind::Learning,
            format!("Document '{}' is ready for visual analysis", document.title),
            format!(
                "The request '{}' can be evaluated without mutating the artwork.",
                request.goal
            ),
        );
        insight.confidence = 1.0;
        insight.preserve = document.ai.intent_locks.preserve.clone();
        insight.evidence.push(Evidence {
            label: format!("{} authored strokes", document.strokes.len()),
            confidence: 1.0,
            region: None,
        });

        let insights = vec![insight];
        AnalysisResult {
            critique: build_critique(&insights, document.ai.intent_locks.preserve.clone()),
            insights,
            ghosts: Vec::new(),
        }
    }
}

/// First useful in-process visual intelligence implementation. It converts
/// authored stroke geometry into explicit evidence and runs deterministic
/// perspective reasoning before producing critique and Ghost Mentor proposals.
#[derive(Debug, Default)]
pub struct DeterministicVisualIntelligence {
    perspective: GeometricPerspectiveInference,
}

impl VisualIntelligence for DeterministicVisualIntelligence {
    fn analyze(&self, document: &Document, request: &AnalysisRequest) -> AnalysisResult {
        let preserve = document.ai.intent_locks.preserve.clone();
        let mut insights = Vec::new();
        let mut ghosts = Vec::new();

        if enabled(request, InsightKind::Perspective) {
            let lines = stroke_line_observations(document);
            if lines.len() >= 2 {
                let solution = self.perspective.infer(&lines, document.projection);
                if let Some(insight) = perspective_insight(&solution, lines.len(), &preserve) {
                    if let Some(ghost) = perspective_ghost(&solution, &insight) {
                        ghosts.push(ghost);
                    }
                    insights.push(insight);
                }
            }
        }

        if enabled(request, InsightKind::Learning) {
            insights.push(process_insight(document, &preserve));
        }

        AnalysisResult {
            critique: build_critique(&insights, preserve),
            insights,
            ghosts,
        }
    }
}

fn enabled(request: &AnalysisRequest, kind: InsightKind) -> bool {
    request.enabled_kinds.contains(&kind)
}

fn stroke_line_observations(document: &Document) -> Vec<LineObservation> {
    document
        .strokes
        .values()
        .filter_map(|stroke| {
            let first = stroke.points.first()?;
            let last = stroke.points.last()?;
            let dx = last.x - first.x;
            let dy = last.y - first.y;
            if dx * dx + dy * dy <= f64::EPSILON {
                return None;
            }
            Some(LineObservation::new(first.x, first.y, last.x, last.y))
        })
        .collect()
}

fn perspective_insight(
    solution: &PerspectiveSolution,
    line_count: usize,
    preserve: &[String],
) -> Option<VisualInsight> {
    if solution.vanishing_points.is_empty() {
        return None;
    }

    let mut insight = VisualInsight::new(
        InsightKind::Perspective,
        format!(
            "Detected {} probable vanishing-point{}",
            solution.vanishing_points.len(),
            if solution.vanishing_points.len() == 1 {
                ""
            } else {
                "s"
            }
        ),
        solution.explanation.clone(),
    );
    insight.confidence = solution.confidence;
    insight.preserve = preserve.to_vec();
    insight.suggested_action = Some(
        "Overlay the inferred perspective guides as a non-destructive Ghost Mentor layer.".into(),
    );
    insight.evidence.push(Evidence {
        label: format!("{line_count} authored stroke directions analyzed"),
        confidence: 1.0,
        region: None,
    });

    for (index, vanishing_point) in solution.vanishing_points.iter().enumerate() {
        insight.evidence.push(Evidence {
            label: format!(
                "VP{} at ({:.2}, {:.2})",
                index + 1,
                vanishing_point.x,
                vanishing_point.y
            ),
            confidence: vanishing_point.confidence,
            region: None,
        });
    }

    if let Some(horizon) = solution.horizon {
        insight.evidence.push(Evidence {
            label: format!("probable horizon y={:.2}", horizon.y),
            confidence: horizon.confidence,
            region: None,
        });
    }

    Some(insight)
}

fn perspective_ghost(
    solution: &PerspectiveSolution,
    insight: &VisualInsight,
) -> Option<GhostSuggestion> {
    if solution.vanishing_points.is_empty() {
        return None;
    }

    let geometry_hint = solution
        .vanishing_points
        .iter()
        .map(|point| (point.x, point.y))
        .collect();

    Some(GhostSuggestion {
        id: Uuid::new_v4(),
        label: "Perspective X-Ray".into(),
        explanation:
            "Shows inferred vanishing points/horizon evidence without modifying authored strokes."
                .into(),
        confidence: solution.confidence,
        source_insight_ids: vec![insight.id],
        geometry_hint,
    })
}

fn process_insight(document: &Document, preserve: &[String]) -> VisualInsight {
    let stroke_count = document.strokes.len();
    let sample_count = document
        .strokes
        .values()
        .map(|stroke| stroke.points.len())
        .sum::<usize>();
    let timed_strokes = document
        .strokes
        .values()
        .filter(|stroke| stroke.duration_ms() > 0)
        .count();

    let mut insight = VisualInsight::new(
        InsightKind::Learning,
        "Artist process data is available for replay and coaching",
        "BlackMamba Paint records stroke samples and time natively, so coaching can be grounded in the authored process instead of guessing from the final image alone.",
    );
    insight.confidence = 1.0;
    insight.preserve = preserve.to_vec();
    insight.evidence = vec![
        Evidence {
            label: format!("{stroke_count} authored strokes"),
            confidence: 1.0,
            region: None,
        },
        Evidence {
            label: format!("{sample_count} captured input samples"),
            confidence: 1.0,
            region: None,
        },
        Evidence {
            label: format!("{timed_strokes} strokes with measurable duration"),
            confidence: 1.0,
            region: None,
        },
    ];
    insight.suggested_action = Some("Enable Artist Replay or generate a targeted exercise.".into());
    insight
}

fn build_critique(insights: &[VisualInsight], do_not_touch: Vec<String>) -> Critique {
    let mut critique = Critique {
        high_impact: Vec::new(),
        medium_impact: Vec::new(),
        low_impact: Vec::new(),
        do_not_touch,
    };

    for insight in insights.iter().cloned() {
        let impact = match insight.kind {
            InsightKind::Perspective
            | InsightKind::Composition
            | InsightKind::Proportion
            | InsightKind::Anatomy => 0.9,
            InsightKind::Value | InsightKind::Edge | InsightKind::Color => 0.7,
            InsightKind::Material
            | InsightKind::Style
            | InsightKind::Context
            | InsightKind::Fashion => 0.5,
            InsightKind::Learning => 0.3,
        } * insight.confidence;

        if impact >= 0.65 {
            critique.high_impact.push(insight);
        } else if impact >= 0.35 {
            critique.medium_impact.push(insight);
        } else {
            critique.low_impact.push(insight);
        }
    }

    critique
}

#[cfg(test)]
mod tests {
    use super::*;
    use bmp_core::{Document, Layer, PointSample, Stroke};

    fn add_line(document: &mut Document, layer_id: Uuid, from: (f64, f64), to: (f64, f64)) {
        document
            .add_stroke(Stroke::new(
                layer_id,
                "construction",
                vec![
                    PointSample {
                        x: from.0,
                        y: from.1,
                        pressure: 0.5,
                        tilt_x: 0.0,
                        tilt_y: 0.0,
                        timestamp_ms: 0,
                    },
                    PointSample {
                        x: to.0,
                        y: to.1,
                        pressure: 0.5,
                        tilt_x: 0.0,
                        tilt_y: 0.0,
                        timestamp_ms: 100,
                    },
                ],
            ))
            .unwrap();
    }

    #[test]
    fn ai_cannot_mutate_document_and_reports_preserve_locks() {
        let mut doc = Document::new("Study");
        doc.ai.intent_locks.preserve = vec!["silhouette".into(), "expression".into()];
        let before = doc.clone();

        let result = MockVisualIntelligence.analyze(
            &doc,
            &AnalysisRequest {
                goal: "critique the drawing".into(),
                enabled_kinds: vec![InsightKind::Perspective, InsightKind::Composition],
            },
        );

        assert_eq!(doc, before);
        assert_eq!(
            result.critique.do_not_touch,
            vec!["silhouette", "expression"]
        );
    }

    #[test]
    fn deterministic_ai_turns_strokes_into_perspective_critique_and_ghost() {
        let mut document = Document::new("Perspective Study");
        document.ai.intent_locks.preserve = vec!["gesture".into()];
        let layer_id = document.add_layer(Layer::new("construction"));
        let left_vp = (-100.0, 50.0);
        let right_vp = (300.0, 50.0);

        for point in [(0.0, 0.0), (30.0, 120.0), (80.0, -20.0)] {
            add_line(&mut document, layer_id, point, left_vp);
        }
        for point in [(20.0, 10.0), (90.0, 130.0), (120.0, -20.0)] {
            add_line(&mut document, layer_id, point, right_vp);
        }

        let before = document.clone();
        let result = DeterministicVisualIntelligence::default().analyze(
            &document,
            &AnalysisRequest {
                goal: "teach me the perspective".into(),
                enabled_kinds: vec![InsightKind::Perspective, InsightKind::Learning],
            },
        );

        assert_eq!(document, before);
        let perspective = result
            .insights
            .iter()
            .find(|insight| insight.kind == InsightKind::Perspective)
            .expect("perspective insight should exist");
        assert!(perspective.confidence > 0.5);
        assert_eq!(perspective.preserve, vec!["gesture"]);
        assert!(!result.ghosts.is_empty());
        assert!(!result.critique.high_impact.is_empty());
        assert_eq!(result.critique.do_not_touch, vec!["gesture"]);

        let why = explain_why(perspective);
        assert!(!why.because.is_empty());
        assert!(why.evidence.iter().any(|item| item.label.contains("VP1")));
    }
}
