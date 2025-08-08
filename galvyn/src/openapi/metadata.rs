//! Openapi related [`RouteMetadata`]

use galvyn_core::router::RouteMetadata;
use std::any::TypeId;

/// Openapi related [`RouteMetadata`]
#[derive(Debug, Clone, Default)]
pub struct OpenapiMetadata {
    pub tags: Vec<&'static str>,
    pub pages: Vec<TypeId>,
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
