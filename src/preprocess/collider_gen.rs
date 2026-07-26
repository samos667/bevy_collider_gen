use crate::{
    abstract_collider::AbstractCollidersBuilder,
    collider_type::ColliderType,
    preprocess::{
        asset_transform::ColliderProcessError,
        combo::{CachedCollider, ColliderAtlas, ColliderSettingsInit},
    },
};
use bevy::prelude::*;

#[allow(clippy::too_many_lines)]
/// Generates a [`CachedCollider`] from an [`Image`] and [`ColliderSettings`] using a compute-intensive sub-app rendering cycle.
pub(crate) fn collider_from_image(
    asset: &Image,
    settings: &ColliderSettingsInit,
) -> Result<CachedCollider, ColliderProcessError> {
    let error_message =
        ColliderProcessError("failed to generate transparency collider for image asset".into());
    if let Some(atlas_init) = &settings.texture_atlas {
        // Convert the Bevy [`Image`] into an `image` crate [`DynamicImage`] so it can be cropped
        let image = asset
            .clone()
            .try_into_dynamic()
            .map_err(|e| ColliderProcessError(e.to_string()))?;
        let atlas = TextureAtlasLayout::from(atlas_init.clone());
        let mut colliders = ColliderAtlas(vec![], atlas.clone());

        // Use collider type from settings to determine which collider type each texture atlas segment should have
        let collider_types = if settings.collider_types.len() == atlas.len() {
            settings.collider_types.clone()
        } else if !settings.collider_types.is_empty()
            && settings
                .collider_types
                .iter()
                .any(std::option::Option::is_some)
        {
            let first =
                settings
                    .collider_types
                    .iter()
                    .flatten()
                    .next()
                    .ok_or(ColliderProcessError(
                        "expected vec to have an element".into(),
                    ))?;
            (0..atlas.len())
                .map(|_| Some(*first))
                .collect::<Vec<Option<ColliderType>>>()
        } else {
            (0..atlas.len())
                .map(|_| Some(default()))
                .collect::<Vec<Option<ColliderType>>>()
        };

        // Generate colliders for each sprite in the atlas's sprite sheet
        for (index, urect) in atlas.textures.iter().enumerate() {
            let sub_image = image.crop_imm(urect.min.x, urect.min.y, urect.width(), urect.height());
            colliders.0.push(
                AbstractCollidersBuilder::from(sub_image)
                    .with_type(
                        collider_types
                            .get(index)
                            .copied()
                            .flatten()
                            .unwrap_or_default(),
                    )
                    .single(),
            );
        }

        Ok(CachedCollider::Atlas(colliders))
    } else if settings.multiple {
        // Create multiple AbstractColliders from an Image
        let colliders = AbstractCollidersBuilder::try_from(asset)
            .ok()
            .map(|builder| {
                let builder = builder
                    .clone()
                    .with_type(
                        settings
                            .collider_types
                            .first()
                            .copied()
                            .flatten()
                            .unwrap_or_default(),
                    )
                    .absolute();
                let image_width = builder.image().width();
                let image_height = builder.image().height();
                let polygons = edges::EdgesIter::new(builder.image());
                polygons
                    .zip(builder.multiple())
                    .filter_map(|(polygon, collider)| {
                        let points = collider.points()?.clone();
                        let pos = polygon.first()?.as_vec2()
                            - points.first()?
                            - Vec2::new((image_width / 2) as f32, (image_height / 2) as f32);
                        Some((collider, pos))
                    })
                    .collect()
            });
        if let Some(colliders) = colliders {
            Ok(CachedCollider::Multiple(colliders))
        } else {
            Err(error_message)
        }
    } else {
        // Create an AbstractCollider from an Image
        if let Some(collider) = AbstractCollidersBuilder::try_from(asset)
            .ok()
            .and_then(|builder| {
                builder
                    .with_type(
                        settings
                            .collider_types
                            .first()
                            .copied()
                            .flatten()
                            .unwrap_or_default(),
                    )
                    .single()
            })
        {
            Ok(CachedCollider::Single(collider))
        } else {
            Err(error_message)
        }
    }
}
