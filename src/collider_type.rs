#[cfg(feature="preprocess")]
use bevy::prelude::*;
#[cfg(feature="preprocess")]
use serde::{Serialize,Deserialize};

use crate::abstract_collider::AbstractCollider;

/// An enumeration representing the different types of colliders that can be created.
#[derive(Clone, Copy, Debug, Default, Hash)]
#[cfg_attr(feature="preprocess", derive(Asset, Reflect, Serialize, Deserialize))]
pub enum ColliderType {
    #[default]
    Polyline,
    ConvexPolyline,
    ConvexHull,
    Heightfield,
}

impl From<AbstractCollider> for ColliderType {
    fn from(value: AbstractCollider) -> Self {
        match value {
            AbstractCollider::Polyline(_) => Self::Polyline,
            AbstractCollider::ConvexPolyline(_) => Self::ConvexPolyline,
            AbstractCollider::ConvexHull(_) => Self::ConvexHull,
            AbstractCollider::Heightfield(..) => Self::Heightfield,
        }
    }
}