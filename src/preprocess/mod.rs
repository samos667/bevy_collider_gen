use crate::preprocess::{
    asset_transform::{ColliderLoader, ColliderSaver, ImageToCollider},
    combo::{CachedCollider, ImageWithCollider},
};
use bevy::{
    asset::{processor::LoadTransformAndSave, transformer::IdentityAssetTransformer},
    platform::collections::HashSet,
    prelude::*,
};

pub mod asset_transform;
pub mod collider_gen;
pub mod combo;

#[derive(Default, Debug)]
/// A [`Plugin`] that allows processing designated [`Image`] files into physics colliders only once.
///
/// Loaded [`Image`]s will be added to the [`World`] within the [`NamedImage`] component for typical use cases.
pub struct PreloadColliderPlugin {
    /// Whether to automatically process `.png` files for collider generation.
    ///
    /// With this set to `false`, you may need to add `.meta` files to images for this plugin to do anything!
    pub set_default_for_png: bool,
}

impl Plugin for PreloadColliderPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<ImageWithCollider>()
            .register_asset_loader(ColliderLoader)
            .register_asset_loader(ImageToCollider)
            .register_asset_processor::<LoadTransformAndSave<
                ImageToCollider,
                IdentityAssetTransformer<ImageWithCollider>,
                ColliderSaver,
            >>(LoadTransformAndSave::new(
                IdentityAssetTransformer::<ImageWithCollider>::new(),
                ColliderSaver,
            ))
            .add_systems(First, init_assets);

        if self.set_default_for_png {
            app.set_default_asset_processor::<LoadTransformAndSave<
                ImageToCollider,
                IdentityAssetTransformer<ImageWithCollider>,
                ColliderSaver,
            >>("png");
        }
    }
}

impl PreloadColliderPlugin {
    #[must_use]
    pub fn without_default_asset_processor(self) -> Self {
        Self {
            set_default_for_png: false,
        }
    }
}

#[derive(Default, Debug, Component, Clone)]
/// A struct that holds loaded [`Image`]s and their generated [`CachedCollider`].
///
/// A [`Name`] [`Component`] contains the original file path for the [`Image`].
pub struct LoadedCollider(pub Handle<Image>, pub Handle<CachedCollider>);

/// Converts newly loaded [`ImageWithCollider`] assets into loaded [`Image`] assets for typical use outside the plugin.
pub fn init_assets(
    mut combo_reader: MessageReader<AssetEvent<ImageWithCollider>>,
    combos: Res<Assets<ImageWithCollider>>,
    mut images: ResMut<Assets<Image>>,
    mut colliders: ResMut<Assets<CachedCollider>>,
    mut commands: Commands,
    mut processed: Local<HashSet<String>>,
) {
    // Unzip the [`ImageWithCollider`] into [`CachedCollider`] and [`Image`] assets
    let loaded_images = combo_reader
        .read()
        .filter_map(|event| match *event {
            AssetEvent::LoadedWithDependencies { id } => combos.get(id).cloned(),
            _ => None,
        })
        .filter_map(
            |ImageWithCollider {
                 collider,
                 source,
                 path,
             }| {
                // Avoid processing this source again (TODO: Allow hot reloads for colliders?)
                if processed.contains(&path) {
                    #[cfg(debug_assertions)]
                    bevy::log::warn!(
                        "Collider at {} was reloaded, but hot reloads aren't yet supported.",
                        path
                    );
                    return None;
                }
                processed.insert(path.clone());

                let collider_handle = colliders.add(collider);
                let image_handle = images.add(source);
                let name = Name::new(path);

                Some((LoadedCollider(image_handle, collider_handle), name))
            },
        )
        .collect::<Vec<(LoadedCollider, Name)>>();

    // Spawn the new loaded images
    for (loaded_image, name) in loaded_images {
        commands.spawn((loaded_image, name));
    }
}
