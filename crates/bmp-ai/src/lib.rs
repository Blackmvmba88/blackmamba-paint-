use bmp_core::Document;
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

/// Provider boundary for local models, cloud models, deterministic analyzers,
/// or future multimodal agents. The provider receives document state and returns
/// proposals. It never receives a mutable Document.
pub trait VisualIntelligence {
    fn analyze(&self, document: &Document, request: &AnalysisRequest) -> AnalysisResult;
}

/// Deterministic development engine. This makes the AI path executable and
/// testable before a specific model provider is wired in.
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

        AnalysisResult {
            insights: vec![insight],
            ghosts: Vec::new(),
            critique: Critique {
                high_impact: Vec::new(),
                medium_impact: Vec::new(),
                low_impact: Vec::new(),
                do_not_touch: document.ai.intent_locks.preserve.clone(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bmp_core::Document;

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
        assert_eq!(result.critique.do_not_touch, vec!["silhouette", "expression"]);
    }
}
