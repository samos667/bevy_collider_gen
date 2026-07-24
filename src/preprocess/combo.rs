use bevy::{image::ImageLoaderSettings, prelude::*};
#[cfg(all(feature = "rapier2d", not(feature = "avian2d")))]
use bevy_rapier2d::prelude::Collider;
#[cfg(all(feature = "avian2d", not(feature = "rapier2d")))]
use avian2d::prelude::Collider;
use serde::{Serialize, Deserialize};
#[cfg(feature = "preprocess")]
use crate::preprocess::asset_transform::ColliderProcessError;
use crate::{
    abstract_collider::AbstractCollider, collider_type::ColliderType,
};
#[cfg(feature = "plugin")]
use crate::plugin::DynamicCollider;

#[derive(Debug, Default, PartialEq, Reflect, Clone, Asset)]
pub struct ImageWithCollider {
    pub collider: SavedCollider,
    pub source: Image,
    pub path: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ImageWithColliderFile {
    pub collider: SavedCollider,
    pub source: Vec<u8>,
}

/// The [`Image`] source of a [`SavedCollider`].
#[derive(Debug, Default, Clone, Reflect, Serialize, Deserialize)]
pub struct ColliderSource(pub String);

/// An asset-generated [`Settings`] `struct` used when generating an [`AbstractCollider`] from an image or texture atlas source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColliderSettings {
    /// A [`Vec`] containing the types of colliders to use for the texture atlas.
    ///
    /// Only uses the first element if not a [`TextureAtlas`].
    #[serde(default)]
    pub collider_types: Vec<ColliderType>,

    /// An optional [`TextureAtlasLayout`] used to manage a sprite sheet.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub texture_atlas: Option<TextureAtlasLayout>,

    #[serde(default)]
    pub image_settings: ImageLoaderSettings,

    pub path: String,
}

impl Default for ColliderSettings {
    fn default() -> Self {
        Self {
            collider_types: vec![ColliderType::Polyline],
            texture_atlas: None,
            image_settings: ImageLoaderSettings::default(),
            path: String::default()
        }
    }
}


/// A user-generated [`Settings`] `struct` used when generating an [`AbstractCollider`] from an image or texture atlas source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColliderSettingsInit {
    /// A [`Vec`] containing the types of colliders to use for the texture atlas.
    ///
    /// Only uses the first element if not a [`TextureAtlas`].
    #[serde(default)]
    pub collider_types: Vec<Option<ColliderType>>,

    /// An optional [`TextureAtlasLayout`] used to manage a sprite sheet.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub texture_atlas: Option<TextureAtlasLayoutInit>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_settings: Option<ImageLoaderSettings>,

    #[serde(default)]
    pub multiple: bool,

    pub path: String,
}

impl Default for ColliderSettingsInit {
    fn default() -> Self {
        Self {
            collider_types: vec![Some(ColliderType::Polyline)],
            texture_atlas: None,
            image_settings: None,
            multiple: false,
            path: String::default()
        }
    }
}

/// An `enum` that either contains a single [`AbstractCollider`] or a collection that matches a [`TextureAtlasLayout`].
#[derive(Debug, Reflect, PartialEq, Asset, Clone, Component, Serialize, Deserialize)]
pub enum SavedCollider {
    /// A single collider generated from an image.
    Single(AbstractCollider),
    /// Multiple colliders generated from an image, along with their offsets.
    Multiple(Vec<(AbstractCollider, Vec2)>),
    /// A sprite sheet/atlas of colliders generated from each section of the source image.
    Atlas(ColliderAtlas),
}

impl Default for SavedCollider {
    fn default() -> Self {
        Self::Single(AbstractCollider::Polyline(vec![]))
    }
}

impl SavedCollider {
    #[cfg(all(feature = "rapier2d", not(feature = "avian2d")))]
    /// Clones this [`SavedCollider`] into a bevy_rapier [`Collider`].
    /// 
    /// Include an `atlas_index` if this is an atlas.
    pub fn to_rapier(&self, atlas_index: Option<usize>) -> Result<Vec<Collider>, ColliderProcessError> {
        let failed = ColliderProcessError("failed to convert AbstractCollider into Rapier Collider".into());
        Ok(match &self {
            SavedCollider::Single(abstract_collider) => vec![abstract_collider.clone().to_rapier().ok_or(failed)?],
            SavedCollider::Multiple(abstract_colliders) => {
                let mut colliders = vec![];
                for (abstract_collider, _) in abstract_colliders.clone() {
                    colliders.push(abstract_collider.to_rapier().ok_or(failed.clone())?);
                }
                colliders
            },
            SavedCollider::Atlas(collider_atlas) => {
                if let Some(index) = atlas_index {
                    vec![
                        collider_atlas.0
                            .get(index)
                            .cloned()
                            .flatten()
                            .ok_or(ColliderProcessError("index out of bounds for collider atlas".into()))?
                            .to_rapier()
                            .ok_or(failed)?
                    ]
                } else { return Err(ColliderProcessError("failed to provide atlas_index for collider atlas".into())); }
            },
        })
    }

    #[cfg(all(feature = "avian2d", not(feature = "rapier2d")))]
    /// Clones this [`SavedCollider`] into an avian [`Collider`].
    /// 
    /// Include an `atlas_index` if this is an atlas.
    pub fn to_avian(&self, atlas_index: Option<usize>) -> Result<Vec<Collider>, ColliderProcessError> {
        let failed = ColliderProcessError("failed to convert AbstractCollider into Rapier Collider".into());
        Ok(match &self {
            SavedCollider::Single(abstract_collider) => vec![abstract_collider.clone().to_avian().ok_or(failed)?],
            SavedCollider::Multiple(abstract_colliders) => {
                let mut colliders = vec![];
                for (abstract_collider, _) in abstract_colliders.clone() {
                    colliders.push(abstract_collider.to_avian().ok_or(failed.clone())?);
                }
                colliders
            },
            SavedCollider::Atlas(collider_atlas) => {
                if let Some(index) = atlas_index {
                    vec![
                        collider_atlas.0
                            .get(index)
                            .cloned()
                            .flatten()
                            .ok_or(ColliderProcessError("index out of bounds for collider atlas".into()))?
                            .to_avian()
                            .ok_or(failed)?
                    ]
                } else { return Err(ColliderProcessError("failed to provide atlas_index for collider atlas".into())); }
            },
        })
    }
}

#[cfg(feature = "plugin")]
impl TryFrom<SavedCollider> for DynamicCollider {
    type Error = ColliderProcessError;

    fn try_from(value: SavedCollider) -> std::prelude::v1::Result<Self, Self::Error> {
        Ok(match value {
            SavedCollider::Single(abstract_collider) => Self {
                collider_type: abstract_collider.into(),
                ..default()
            },
            SavedCollider::Multiple(_) => {
                return Err(ColliderProcessError("multiple colliders cannot become one DynamicCollider".into()));
            },
            SavedCollider::Atlas(collider_atlas) => Self {
                collider_type: collider_atlas.0.iter()
                    .flatten()
                    .next()
                    .map(|item| item.clone().into())
                    .unwrap_or(default()),
                ..default()
            },
        })
    }
}

/// A `struct` containing [`AbstractCollider`]s in the same indices as its corresponding [`TextureAtlasLayout`].
/// 
/// Index into this `struct` to get the [`AbstractCollider`] for the active [`Sprite`] image in a sprite sheet.
#[derive(Debug, Reflect, PartialEq, Asset, Clone, Serialize, Deserialize)]
pub struct ColliderAtlas(pub Vec<Option<AbstractCollider>>, pub TextureAtlasLayout);

/// Contains compact data for initializing a [`TextureAtlasLayout`] `struct`.
/// 
/// Adapted from `bevy_asset_loader`.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Asset, Reflect)]
pub struct TextureAtlasLayoutInit {
    /// The image width in pixels
    pub tile_size_x: u32,
    /// The image height in pixels
    pub tile_size_y: u32,
    /// Columns on the sprite sheet
    pub columns: u32,
    /// Rows on the sprite sheet
    pub rows: u32,
    /// Padding between columns in pixels
    #[serde(skip_serializing_if = "Option::is_none")]
    pub padding_x: Option<u32>,
    /// Padding between rows in pixels
    #[serde(skip_serializing_if = "Option::is_none")]
    pub padding_y: Option<u32>,
    /// Number of pixels offset of the first tile
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset_x: Option<u32>,
    /// Number of pixels offset of the first tile
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset_y: Option<u32>,
}

impl From<TextureAtlasLayoutInit> for TextureAtlasLayout {
    fn from(val: TextureAtlasLayoutInit) -> Self {
        TextureAtlasLayout::from_grid(
            UVec2::new(val.tile_size_x, val.tile_size_y),
            val.columns,
            val.rows,
            Some(UVec2::new(val.padding_x.unwrap_or(0), val.padding_y.unwrap_or(0))),
            Some(UVec2::new(val.offset_x.unwrap_or(0), val.offset_y.unwrap_or(0))),
        )
    }
}