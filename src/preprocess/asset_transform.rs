use bevy::{asset::{
    AssetLoader,
    AsyncWriteExt,
    LoadContext,
    io::{Reader, Writer},
    saver::{AssetSaver, SavedAsset},
}, image::{CompressedImageFormats, ImageArrayLayout, ImageFormatSetting, ImageLoaderSettings, ImageType}, prelude::*, render::render_resource::TextureFormat};
use image::ExtendedColorType;
use ron::{Error, ser::PrettyConfig};
use crate::preprocess::{
        collider_gen::collider_from_image, combo::{ImageWithCollider, ImageWithColliderFile, ColliderSettings, ColliderSettingsInit, SavedCollider}};

/// An [`AssetLoader`] that transforms an [`Image`] into an [`AbstractCollider`], loading both as assets.
#[derive(Debug, Default, Reflect)]
pub struct ImageToCollider;

impl AssetLoader for ImageToCollider {
    type Asset = ImageWithCollider;
    type Settings = ColliderSettingsInit;
    type Error = ColliderProcessError;

    async fn load(
        &self,
        reader: &mut dyn Reader,
        settings: &Self::Settings,
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let source = bevy::image::ImageLoader::new(default())
            .load(reader, &bevy::image::ImageLoaderSettings::default(), load_context).await;
        let image = source.map_err(|e| ColliderProcessError(e.to_string()))?;
        let collider = collider_from_image(&image, &settings);
        let path = load_context
            .path()
            .path()
            .to_str()
            .ok_or(ColliderProcessError("failed to convert file path to unicode".into()))?
            .to_owned();
        Ok(ImageWithCollider {
            collider: collider?,
            source: image,
            path
        })
    }
}

/// Loads a [`SavedColliders`] struct from file.
#[derive(Debug, Default, Reflect)]
pub struct ColliderLoader;

impl AssetLoader for ColliderLoader {
    type Asset = ImageWithCollider;
    type Settings = ColliderSettings;
    type Error = Error;

     async fn load(
        &self,
        reader: &mut dyn Reader,
        settings: &Self::Settings,
        load_context: &mut LoadContext<'_>,
    ) -> Result<ImageWithCollider, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let mut file = ron::de::from_bytes::<ImageWithColliderFile>(&bytes)?;
        let source = Self::load_image(&mut file.source, &settings.image_settings, load_context).map_err(|e| Error::Message(e.to_string()))?;
        let path = load_context
            .path()
            .path()
            .to_str()
            .ok_or(Error::Message("failed to convert file path to unicode".into()))?
            .to_owned();
        Ok(ImageWithCollider { collider: file.collider, source, path })
    }

    fn extensions(&self) -> &[&str] {
        &["ron"]
    }
}

impl ColliderLoader {
    /// A copy of `bevy_image`'s [`AssetLoader`] `load` function, but without certain features due to private `struct`s.
    /// 
    /// TODO: After Bevy 0.19 update, save [`Image`] separately to allow more versatility for users (& save file space)
    fn load_image(bytes: &mut Vec<u8>, settings: &ImageLoaderSettings, load_context: &mut LoadContext<'_>) -> Result<Image, ColliderProcessError> {
        // use the file extension for the image type
        let ext = load_context
            .path()
            .path()
            .extension()
            .ok_or(ColliderProcessError("image has no file extension".into()))?
            .to_str()
            .ok_or(ColliderProcessError("image file extension is not unicode".into()))?;
        Self::image_from_bytes(ext, bytes, settings)
    }

    fn image_from_bytes(ext: &str, bytes: &mut Vec<u8>, settings: &ImageLoaderSettings) -> Result<Image, ColliderProcessError> {
        let image_type = match settings.format {
            // Due to private fields, matching by extension is all that is supported here
            _ => {
                ImageType::Extension(ext)
            }
        };
        let mut image = Image::from_buffer(
            &bytes,
            image_type,
            CompressedImageFormats::default(),
            settings.is_srgb,
            settings.sampler.clone(),
            settings.asset_usage,
        )
        .map_err(|e| ColliderProcessError(e.to_string()))?;

        if let Some(format) = settings.texture_format {
            image.texture_descriptor.format = format;
        }

        if let Some(array_layout) = settings.array_layout {
            let layers = match array_layout {
                ImageArrayLayout::RowCount { rows } => rows,
                ImageArrayLayout::RowHeight { pixels } => image.height() / pixels,
            };

            image.reinterpret_stacked_2d_as_array(layers).map_err(|e| ColliderProcessError(e.to_string()))?;
        }

        Ok(image)
    }
}

#[derive(Debug, Default, Reflect)]
pub struct ColliderSaver;

impl AssetSaver for ColliderSaver {
    type Asset = ImageWithCollider;
    type Settings = ();
    type OutputLoader = ColliderLoader;
    type Error = Error;

    async fn save(
        &self,
        writer: &mut Writer,
        asset: SavedAsset<'_, Self::Asset>,
        _settings: &Self::Settings,
    ) -> Result<ColliderSettings, Self::Error> {
        let image_ref = &asset.get().source;
        let (image_raw, load_settings) = Self::image_to_bytes(
            image_ref,
            default(),
            "png",
        ).map_err(|e| Error::Message(e.to_string()))?;
        let collider_combo_file = ImageWithColliderFile {
            collider: asset.get().collider.clone(),
            source: image_raw,
        };
        let file_ron = ron::ser::to_string_pretty(&collider_combo_file, PrettyConfig::default().compact_arrays(true))?;
        writer.write_all(&file_ron.into_bytes()).await?;
        Ok(ColliderSettings {
            collider_types: vec![], // TODO: Get these from collider
            texture_atlas: if let Some(atlas) = match collider_combo_file.collider {
                SavedCollider::Atlas(collider_atlas) => Some(collider_atlas.1),
                _ => None,
            } {
                Some(atlas)
            } else { None },
            image_settings: load_settings,
            path: asset.get().path.clone(),
        })
    }
}

impl ColliderSaver {
    /// Saves the provided [`Image`] according to its [`ImageSaverSettings`].
    /// 
    /// Code from `bevy_image`'s [`ImageSaver`] function.
    fn image_to_bytes(asset: &Image, settings: ImageSaverSettings, extension: &str) -> Result<(Vec<u8>, ImageLoaderSettings), ColliderProcessError> {
        let format = match settings.format {
            SaveImageFormatSetting::Format(format) => format,
            SaveImageFormatSetting::FromExtension => {
                ImageFormat::from_extension(extension)
                    .ok_or_else(|| ColliderProcessError("unknown extension".into()))?
            },
        };

        let Some(_asset_data) = asset.data.as_ref() else {
            return Err(ColliderProcessError("image is missing data".into()));
        };

        // TODO: Consider supporting more formats here!
        let (image_crate_format, color_type, is_srgb): (_, ExtendedColorType, _) = match format {
            ImageFormat::Png => match asset.texture_descriptor.format {
                TextureFormat::R8Unorm => (image::ImageFormat::Png, ExtendedColorType::L8, false),
                TextureFormat::Rgba8Unorm => {
                    (image::ImageFormat::Png, ExtendedColorType::Rgba8, false)
                }
                TextureFormat::Rgba8UnormSrgb => {
                    (image::ImageFormat::Png, ExtendedColorType::Rgba8, true)
                }
                _ => {
                    return Err(ColliderProcessError("unsupported texture format".into()))
                }
            },
            _ => return Err(ColliderProcessError("unsupported format".into())),
        };

        let mut bytes = Vec::<u8>::new();
        image::write_buffer_with_format(
            &mut std::io::Cursor::new(&mut bytes),
            _asset_data,
            asset.width(),
            asset.height(),
            color_type,
            image_crate_format,
        ).map_err(|e| ColliderProcessError(e.to_string()))?;

        Ok((
            bytes.to_vec(),
            ImageLoaderSettings {
                format: ImageFormatSetting::Format(format),
                // Passing in the original texture format breaks things. For example, PNG will save R8
                // data as RGBA8 data: if we later try to load as R8, we get 4 times as many pixels!
                texture_format: None,
                is_srgb,
                sampler: asset.sampler.clone(),
                asset_usage: asset.asset_usage,
                array_layout: None,
            },
        ))
    }
}

/// A simple error type for wrapping other crates' errors.
#[derive(Debug, Default, Clone)]
pub struct ColliderProcessError(pub String);

impl std::fmt::Display for ColliderProcessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for ColliderProcessError {}


#[derive(Default, Debug, Clone)]
/// Settings for how to save an image.
pub struct ImageSaverSettings {
    /// Defines the file format that the image will be saved as.
    pub format: SaveImageFormatSetting,
}

#[derive(Default, Debug, Clone)]
pub enum SaveImageFormatSetting {
    /// The file format to write will be deduced from the file path being written to.
    #[default]
    FromExtension,
    /// This is the explicit file format being written.
    Format(ImageFormat),
}


#[allow(unused_imports)]
mod test {
    use std::io::Read;
    use bevy::asset::{meta::AssetMeta, processor::LoadTransformAndSave, transformer::IdentityAssetTransformer};
    use super::*;

    /// Tests that Bevy [`Image`]s can be converted to and from the `image` crate structs across Bevy versions.
    #[test]
    fn image_convert_test() -> Result<(), ColliderProcessError> {
        let file = std::fs::File::open("assets/sprite/car.png").expect("image file to exist");
        let buffered_file = std::io::BufReader::new(file);
        let test_image = Image::from_dynamic(image::load(buffered_file, image::ImageFormat::Png).expect("original image loads"), true, default());
        let (mut raw_image, settings) = ColliderSaver::image_to_bytes(&test_image, default(), "png")?;
        let reloaded_image = ColliderLoader::image_from_bytes("png", &mut raw_image, &settings)?;
        assert_eq!(test_image, reloaded_image);
        Ok(())
    }

    /// Tests [`Image`] loading using an actual image file rather than a generated test [`Image`].
    #[test]
    fn image_load_test() -> Result<(), ColliderProcessError> {
        let file = std::fs::File::open("assets/sprite/car.png").expect("image file to exist");
        let buffered_file = std::io::BufReader::new(file);
        let original_image = image::load(buffered_file, image::ImageFormat::Png).expect("original image loads");
        let bevy_image = Image::from_dynamic(original_image, true, default());
        let mut file = std::fs::File::open("assets/sprite/car.png").expect("image file to exist");
        // let buffered_file = std::io::BufReader::new(file);
        let mut bytes = vec![];
        std::io::Read::read_to_end(&mut file, &mut bytes).expect("can read file");
        let loaded_image = ColliderLoader::image_from_bytes("png", &mut bytes, &default()).expect("image loads with crate");
        assert_eq!(bevy_image, loaded_image);
        Ok(())
    }

    /// Tests [`Image`] embedding in the serialized [`ImageWithColliderFile`] struct to ensure it produces valid [`Image`] instances.
    #[test]
    fn image_serialize_test() -> Result<(), ColliderProcessError> {
        let file = std::fs::File::open("assets/sprite/car.png").expect("image file to exist");
        let buffered_file = std::io::BufReader::new(file);
        let original_image = image::load(buffered_file, image::ImageFormat::Png).expect("original image loads");
        let bevy_image = Image::from_dynamic(original_image, true, default());
        let (mut bytes, settings) = ColliderSaver::image_to_bytes(&bevy_image, default(), "png").expect("image to become bytes");
        let reloaded_image = ColliderLoader::image_from_bytes("png", &mut bytes, &settings).expect("image to reload");
        let combo = ImageWithColliderFile {
            collider: default(),
            source: bytes
        };
        let serialized = ron::ser::to_string_pretty(&combo, PrettyConfig::default().compact_arrays(true)).expect("combo file serializes");
        let mut deserialized: ImageWithColliderFile = ron::de::from_str(&serialized).expect("deserializes");
        let reworked_image = ColliderLoader::image_from_bytes("png", &mut deserialized.source, &settings).expect("image to rework");
        assert_eq!(reworked_image, reloaded_image);
        assert_eq!(reloaded_image, bevy_image);
        Ok(())
    }

    /// Verifies that the imported asset matches the original [`Image`].
    #[test]
    fn imported_asset_verification() -> Result<(), ColliderProcessError> {
        // Load original version
        let file = std::fs::File::open("assets/sprite_with_meta/car.png").expect("image file to exist");
        let buffered_file = std::io::BufReader::new(file);
        let dynamic_image = image::load(buffered_file, image::ImageFormat::Png).expect("original image loads");
        let bevy_image = Image::from_dynamic(dynamic_image, true, default());

        // Load processed version
        let file = std::fs::File::open("imported_assets/Default/sprite_with_meta/car.png").expect("image file to exist");
        let mut buffered_file = std::io::BufReader::new(file);
        let mut serialized = String::new();
        buffered_file.read_to_string(&mut serialized).expect("buffered file reads");
        let mut collider_combo: ImageWithColliderFile = ron::de::from_str(&serialized).expect("combo deserializes");

        // Load settings
        let file = std::fs::File::open("imported_assets/Default/sprite_with_meta/car.png.meta").expect("image file to exist");
        let mut buffered_file = std::io::BufReader::new(file);
        let mut serialized = String::new();
        buffered_file.read_to_string(&mut serialized).expect("buffered file reads");
        let collider_meta: AssetMeta::<ColliderLoader, LoadTransformAndSave<ImageToCollider, IdentityAssetTransformer<ImageWithCollider>, ColliderSaver>> = ron::de::from_str(&serialized).expect("settings deserialize");
        let collider_settings = match collider_meta.asset {
            bevy::asset::meta::AssetAction::Load { settings, .. } => settings,
            _ => panic!("incorrect asset action"),
        };

        // Check if equal
        let processed_image = ColliderLoader::image_from_bytes("png", &mut collider_combo.source, &collider_settings.image_settings).expect("image generates from bytes");
        assert_eq!(bevy_image, processed_image);
        Ok(())
    }
}