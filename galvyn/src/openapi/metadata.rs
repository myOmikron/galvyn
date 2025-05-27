//! Openapi related [`RouteMetadata`]

use galvyn_core::router::RouteMetadata;

/// Openapi related [`RouteMetadata`]
#[derive(Debug, Clone, Default)]
pub struct OpenapiMetadata {
    pub tags: Vec<&'static str>,
}

impl RouteMetadata for OpenapiMetadata {
    fn merge(&mut self, other: &Self) {
        for tag in &other.tags {
            if !self.tags.contains(tag) {
                self.tags.push(tag);
            }
        }
    }
}
