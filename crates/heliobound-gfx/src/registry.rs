use crate::scene::TextStyle;
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub struct VisualDefinition {
    pub glyph: char,
    pub style: TextStyle,
    pub label: String,
}

#[derive(Clone, Debug, Default)]
pub struct VisualRegistry {
    visuals: BTreeMap<String, VisualDefinition>,
}

impl VisualRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, key: impl Into<String>, visual: VisualDefinition) {
        self.visuals.insert(key.into(), visual);
    }

    pub fn resolve<'a>(&'a self, candidates: &[&str]) -> &'a VisualDefinition {
        for candidate in candidates {
            if let Some(visual) = self.visuals.get(*candidate) {
                return visual;
            }
        }

        self.visuals
            .get("unknown")
            .expect("visual registry must define an 'unknown' fallback")
    }
}

impl VisualDefinition {
    pub fn simple(glyph: char, label: impl Into<String>) -> Self {
        Self {
            glyph,
            label: label.into(),
            style: TextStyle::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_exact_match_before_unknown() {
        let mut registry = VisualRegistry::new();
        registry.register("unknown", VisualDefinition::simple('?', "unknown"));
        registry.register("region/default", VisualDefinition::simple('R', "region"));
        registry.register(
            "region/kind/planet",
            VisualDefinition::simple('P', "planet"),
        );

        let visual = registry.resolve(&["region/kind/planet", "region/default", "unknown"]);
        assert_eq!(visual.glyph, 'P');
    }

    #[test]
    fn falls_back_to_unknown() {
        let mut registry = VisualRegistry::new();
        registry.register("unknown", VisualDefinition::simple('?', "unknown"));

        let visual = registry.resolve(&["missing/thing", "still/missing"]);
        assert_eq!(visual.glyph, '?');
    }
}
