use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ConceptKind {
    Technique,
    Style,
    Material,
    Texture,
    ColorTheory,
    Artist,
    Movement,
    Fashion,
    Architecture,
    Composition,
    Context,
    Anatomy,
    Photography,
    Cinema,
    ProductDesign,
    Symbol,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Concept {
    pub id: Uuid,
    pub name: String,
    pub kind: ConceptKind,
    pub description: String,
    pub tags: BTreeSet<String>,
}

impl Concept {
    pub fn new(name: impl Into<String>, kind: ConceptKind) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            kind,
            description: String::new(),
            tags: BTreeSet::new(),
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.insert(normalize(tag.into()));
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RelationKind {
    Influences,
    Uses,
    SimilarTo,
    ContrastsWith,
    AppearsIn,
    Requires,
    AssociatedWith,
    DerivedFrom,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Relation {
    pub id: Uuid,
    pub from: Uuid,
    pub to: Uuid,
    pub kind: RelationKind,
    pub strength: f32,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct KnowledgeGraph {
    pub concepts: BTreeMap<Uuid, Concept>,
    pub relations: BTreeMap<Uuid, Relation>,
}

impl KnowledgeGraph {
    pub fn add_concept(&mut self, concept: Concept) -> Uuid {
        let id = concept.id;
        self.concepts.insert(id, concept);
        id
    }

    pub fn relate(
        &mut self,
        from: Uuid,
        to: Uuid,
        kind: RelationKind,
        strength: f32,
    ) -> Result<Uuid, KnowledgeError> {
        self.require_concept(from)?;
        self.require_concept(to)?;
        let relation = Relation {
            id: Uuid::new_v4(),
            from,
            to,
            kind,
            strength: strength.clamp(0.0, 1.0),
            note: None,
        };
        let id = relation.id;
        self.relations.insert(id, relation);
        Ok(id)
    }

    pub fn related(&self, concept_id: Uuid) -> Result<Vec<&Concept>, KnowledgeError> {
        self.require_concept(concept_id)?;
        Ok(self
            .relations
            .values()
            .filter_map(|relation| {
                if relation.from == concept_id {
                    self.concepts.get(&relation.to)
                } else if relation.to == concept_id {
                    self.concepts.get(&relation.from)
                } else {
                    None
                }
            })
            .collect())
    }

    pub fn search(&self, query: &str) -> Vec<&Concept> {
        let query = normalize(query.to_string());
        if query.is_empty() {
            return Vec::new();
        }
        self.concepts
            .values()
            .filter(|concept| {
                normalize(concept.name.clone()).contains(&query)
                    || normalize(concept.description.clone()).contains(&query)
                    || concept.tags.iter().any(|tag| tag.contains(&query))
            })
            .collect()
    }

    fn require_concept(&self, id: Uuid) -> Result<(), KnowledgeError> {
        self.concepts
            .contains_key(&id)
            .then_some(())
            .ok_or(KnowledgeError::MissingConcept(id))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReferenceRole {
    Pose,
    Color,
    Material,
    Texture,
    Fashion,
    Lighting,
    Composition,
    Architecture,
    Anatomy,
    Context,
    Style,
    Perspective,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArtReference {
    pub id: Uuid,
    pub label: String,
    pub source: Option<String>,
    pub roles: BTreeSet<ReferenceRole>,
    pub concept_ids: BTreeSet<Uuid>,
    pub note: Option<String>,
}

impl ArtReference {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            label: label.into(),
            source: None,
            roles: BTreeSet::new(),
            concept_ids: BTreeSet::new(),
            note: None,
        }
    }

    pub fn role(mut self, role: ReferenceRole) -> Self {
        self.roles.insert(role);
        self
    }

    pub fn concept(mut self, concept_id: Uuid) -> Self {
        self.concept_ids.insert(concept_id);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StyleGenome {
    pub traits: BTreeMap<String, f32>,
    pub descriptors: BTreeSet<String>,
}

impl Default for StyleGenome {
    fn default() -> Self {
        Self {
            traits: BTreeMap::new(),
            descriptors: BTreeSet::new(),
        }
    }
}

impl StyleGenome {
    pub fn set_trait(&mut self, name: impl Into<String>, value: f32) -> Result<(), KnowledgeError> {
        if !value.is_finite() || !(0.0..=1.0).contains(&value) {
            return Err(KnowledgeError::InvalidGenomeValue(value));
        }
        self.traits.insert(normalize(name.into()), value);
        Ok(())
    }

    pub fn descriptor(&mut self, descriptor: impl Into<String>) {
        self.descriptors.insert(normalize(descriptor.into()));
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RuleStrength {
    Suggestion,
    Canon,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorldRule {
    pub id: Uuid,
    pub key: String,
    pub value: String,
    pub rationale: Option<String>,
    pub strength: RuleStrength,
}

impl WorldRule {
    pub fn new(key: impl Into<String>, value: impl Into<String>, strength: RuleStrength) -> Self {
        Self {
            id: Uuid::new_v4(),
            key: key.into(),
            value: value.into(),
            rationale: None,
            strength,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ArtworkKnowledge {
    pub concept_ids: BTreeSet<Uuid>,
    pub references: BTreeMap<Uuid, ArtReference>,
    pub style_genome: StyleGenome,
    pub world_rules: BTreeMap<Uuid, WorldRule>,
}

impl ArtworkKnowledge {
    pub fn attach_concept(
        &mut self,
        graph: &KnowledgeGraph,
        concept_id: Uuid,
    ) -> Result<(), KnowledgeError> {
        graph.require_concept(concept_id)?;
        self.concept_ids.insert(concept_id);
        Ok(())
    }

    pub fn add_reference(
        &mut self,
        graph: &KnowledgeGraph,
        reference: ArtReference,
    ) -> Result<Uuid, KnowledgeError> {
        for concept_id in &reference.concept_ids {
            graph.require_concept(*concept_id)?;
        }
        let id = reference.id;
        self.references.insert(id, reference);
        Ok(id)
    }

    pub fn references_for_role(&self, role: &ReferenceRole) -> Vec<&ArtReference> {
        self.references
            .values()
            .filter(|reference| reference.roles.contains(role))
            .collect()
    }

    pub fn add_world_rule(&mut self, rule: WorldRule) -> Uuid {
        let id = rule.id;
        self.world_rules.insert(id, rule);
        id
    }
}

fn normalize(value: String) -> String {
    value.trim().to_lowercase()
}

#[derive(Debug, Error, PartialEq)]
pub enum KnowledgeError {
    #[error("knowledge graph does not contain concept {0}")]
    MissingConcept(Uuid),
    #[error("style genome values must be finite and between 0 and 1, got {0}")]
    InvalidGenomeValue(f32),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn concepts_form_cross_domain_art_relationships() {
        let mut graph = KnowledgeGraph::default();
        let bauhaus = graph.add_concept(
            Concept::new("Bauhaus", ConceptKind::Movement)
                .tag("geometry")
                .tag("design"),
        );
        let architecture = graph.add_concept(Concept::new(
            "Modernist architecture",
            ConceptKind::Architecture,
        ));
        let color = graph.add_concept(Concept::new(
            "Primary color systems",
            ConceptKind::ColorTheory,
        ));
        graph
            .relate(bauhaus, architecture, RelationKind::Influences, 0.9)
            .unwrap();
        graph
            .relate(bauhaus, color, RelationKind::AssociatedWith, 0.8)
            .unwrap();

        let related = graph.related(bauhaus).unwrap();
        assert_eq!(related.len(), 2);
        assert_eq!(graph.search("geometry").len(), 1);
    }

    #[test]
    fn one_reference_can_have_explicit_independent_roles() {
        let graph = KnowledgeGraph::default();
        let mut artwork = ArtworkKnowledge::default();
        let reference = ArtReference::new("night street reference")
            .role(ReferenceRole::Lighting)
            .role(ReferenceRole::Color);
        artwork.add_reference(&graph, reference).unwrap();

        assert_eq!(
            artwork.references_for_role(&ReferenceRole::Lighting).len(),
            1
        );
        assert_eq!(artwork.references_for_role(&ReferenceRole::Pose).len(), 0);
    }

    #[test]
    fn style_genome_is_parameterized_instead_of_named_only() {
        let mut genome = StyleGenome::default();
        genome.set_trait("shape angularity", 0.82).unwrap();
        genome.set_trait("detail density", 0.91).unwrap();
        genome.descriptor("industrial");

        assert_eq!(genome.traits["shape angularity"], 0.82);
        assert!(genome.descriptors.contains("industrial"));
    }

    #[test]
    fn world_bible_can_distinguish_suggestions_from_canon() {
        let mut artwork = ArtworkKnowledge::default();
        artwork.add_world_rule(WorldRule::new(
            "warning color",
            "white",
            RuleStrength::Canon,
        ));
        artwork.add_world_rule(WorldRule::new(
            "ornament density",
            "low",
            RuleStrength::Suggestion,
        ));

        assert_eq!(artwork.world_rules.len(), 2);
        assert!(artwork
            .world_rules
            .values()
            .any(|rule| rule.strength == RuleStrength::Canon));
    }
}
