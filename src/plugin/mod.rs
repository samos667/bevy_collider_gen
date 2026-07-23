use bevy::{asset::LoadState, prelude::*};

use crate::prelude::AbstractCollider;
pub use components::DynamicCollider;
use systems::update_colliders;

pub mod components;
pub mod systems;
pub(crate) mod utils;

#[derive(Debug, Default)]
pub struct DynamicColliderPlugin<Target>(std::marker::PhantomData<Target>)
where
    Target: Component;

impl<Target> DynamicColliderPlugin<Target>
where
    Target: Component,
{
    #[must_use]
    pub fn new() -> DynamicColliderPlugin<Target> {
        DynamicColliderPlugin(std::marker::PhantomData)
    }
}

impl<TargetCollider> bevy::prelude::Plugin for DynamicColliderPlugin<TargetCollider>
where
    AbstractCollider: Into<Option<TargetCollider>>,
    TargetCollider: Component,
{
    fn build(&self, app: &mut App) {
        app.add_systems(
            Last,
            update_colliders::<TargetCollider>.run_if(assets_loaded),
        );
    }
}

fn assets_loaded(
    asset_server: Res<AssetServer>,
    targets: Query<(&DynamicCollider, Option<&Sprite>)>,
) -> bool {
    targets
        .iter()
        .map(|(source, sprite)| source.image.as_ref().or(sprite.map(|sprite| &sprite.image)))
        .all(|handle| {
            handle.is_some_and(|handle| {
                // If there's no load state, that means the [`Image`] has been manually added (& thus is loaded already)
                if let Some(load_state) = asset_server.get_load_state(handle) {
                    matches!(load_state, LoadState::Loaded)
                } else { true }
            })
        })
}
