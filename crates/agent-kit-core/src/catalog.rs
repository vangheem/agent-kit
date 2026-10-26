use std::collections::BTreeMap;
use std::sync::Arc;

use crate::action::{Action, ActionDescriptor};

#[async_trait::async_trait]
pub trait ActionCatalog: Send + Sync {
    fn list(&self) -> Vec<ActionDescriptor>;
    fn get(&self, id: &str) -> Option<Arc<dyn Action>>;
    fn search(&self, query: &str, limit: usize) -> Vec<ActionDescriptor>;
}

pub struct MapCatalog {
    actions: BTreeMap<String, Arc<dyn Action>>,
}

impl MapCatalog {
    pub fn new(actions: Vec<Arc<dyn Action>>) -> Self {
        let mut map = BTreeMap::new();
        for action in actions {
            let id = action.descriptor().id;
            map.insert(id, action);
        }
        Self { actions: map }
    }
}

impl std::fmt::Debug for MapCatalog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MapCatalog")
            .field("ids", &self.actions.keys().collect::<Vec<_>>())
            .finish()
    }
}

#[async_trait::async_trait]
impl ActionCatalog for MapCatalog {
    fn list(&self) -> Vec<ActionDescriptor> {
        self.actions
            .values()
            .map(|action| action.descriptor())
            .collect()
    }

    fn get(&self, id: &str) -> Option<Arc<dyn Action>> {
        self.actions.get(id).cloned()
    }

    fn search(&self, query: &str, limit: usize) -> Vec<ActionDescriptor> {
        let limit = limit.clamp(1, 50);
        if query.is_empty() {
            return self.list().into_iter().take(limit).collect();
        }
        if let Some(action) = self.actions.get(query) {
            return vec![action.descriptor()];
        }
        let tokens: Vec<String> = query
            .split_whitespace()
            .map(|token| token.to_lowercase())
            .collect();
        let mut scored: Vec<(usize, ActionDescriptor)> = self
            .actions
            .values()
            .filter_map(|action| {
                let descriptor = action.descriptor();
                let score = score_descriptor(&descriptor, &tokens);
                (score > 0).then_some((score, descriptor))
            })
            .collect();
        scored.sort_by(|left, right| {
            right
                .0
                .cmp(&left.0)
                .then_with(|| left.1.id.cmp(&right.1.id))
        });
        scored
            .into_iter()
            .take(limit)
            .map(|(_, descriptor)| descriptor)
            .collect()
    }
}

fn score_descriptor(descriptor: &ActionDescriptor, tokens: &[String]) -> usize {
    let id = descriptor.id.to_lowercase();
    let title = descriptor.title.to_lowercase();
    let summary = descriptor.summary.to_lowercase();
    tokens
        .iter()
        .filter(|token| {
            id.contains(token.as_str())
                || title.contains(token.as_str())
                || summary.contains(token.as_str())
        })
        .count()
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::MapCatalog;
    use crate::action::{Action, ActionDescriptor, ActionError, ActionOutput, Effect};
    use crate::catalog::ActionCatalog;

    struct Stub {
        id: &'static str,
        title: &'static str,
        summary: &'static str,
    }

    #[async_trait::async_trait]
    impl Action for Stub {
        fn descriptor(&self) -> ActionDescriptor {
            ActionDescriptor {
                id: self.id.to_owned(),
                title: self.title.to_owned(),
                summary: self.summary.to_owned(),
                input_schema: serde_json::json!({}),
                output_schema: serde_json::json!({}),
                requires: Vec::new(),
                effect: Effect::Read,
            }
        }

        async fn execute(
            &self,
            _ctx: &crate::action::ActionContext,
            _input: serde_json::Value,
        ) -> Result<ActionOutput, ActionError> {
            Err(ActionError::Internal {
                message: "unused".to_owned(),
            })
        }
    }

    #[test]
    fn search_exact_id_and_token_rank() {
        let catalog = MapCatalog::new(vec![
            Arc::new(Stub {
                id: "markets.quote",
                title: "Quote",
                summary: "latest price",
            }),
            Arc::new(Stub {
                id: "web.search",
                title: "Web widgets",
                summary: "find pages",
            }),
            Arc::new(Stub {
                id: "shop.search",
                title: "Shop",
                summary: "widgets catalog extra",
            }),
        ]);

        let exact = catalog.search("web.search", 10);
        assert_eq!(exact.len(), 1);
        assert_eq!(exact[0].id, "web.search");

        let ranked = catalog.search("widgets catalog", 10);
        assert_eq!(ranked[0].id, "shop.search");
        assert_eq!(ranked[1].id, "web.search");
    }
}
