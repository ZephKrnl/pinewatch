//! `LandscapeMaterialAsset` (`Bundles/LandscapeMaterialAsset.cs`): a terrain layer's textures, physics material,
//! foliage, automatic painting rules and holiday redirects. Its `PopulateAsset` is ported whole; building the Unity
//! terrain layer comes with the terrain materials (M2e).

use pinewatch_dat::DictRef;
use pinewatch_mono::{MonoException, NetEnum, NetString, try_parse_enum};

use crate::holiday::ENPCHoliday;
use crate::references::{AssetReference, ContentReference};

pinewatch_mono::net_enum! {
    /// `EPhysicsMaterial` (`Tools/EPhysicsMaterial.cs`), the legacy physics material.
    pub struct EPhysicsMaterial(i32) {
        NONE = 0,
        CLOTH_DYNAMIC = 1,
        CLOTH_STATIC = 2,
        TILE_DYNAMIC = 3,
        TILE_STATIC = 4,
        CONCRETE_DYNAMIC = 5,
        CONCRETE_STATIC = 6,
        FLESH_DYNAMIC = 7,
        GRAVEL_DYNAMIC = 8,
        GRAVEL_STATIC = 9,
        METAL_DYNAMIC = 10,
        METAL_STATIC = 11,
        METAL_SLIP = 12,
        WOOD_DYNAMIC = 13,
        WOOD_STATIC = 14,
        FOLIAGE_STATIC = 15,
        FOLIAGE_DYNAMIC = 16,
        SNOW_STATIC = 17,
        ICE_STATIC = 18,
        WATER_STATIC = 19,
        ALIEN_DYNAMIC = 20,
        SAND_STATIC = 21,
    }
}

impl EPhysicsMaterial {
    /// `PhysicsTool.GetNameOfLegacyMaterial`: the physics material name for a legacy value; null for `NONE` and for
    /// values no member has.
    pub fn legacy_name(self) -> Option<&'static str> {
        Some(match self {
            Self::CLOTH_DYNAMIC | Self::CLOTH_STATIC => "Cloth",
            Self::TILE_DYNAMIC | Self::TILE_STATIC => "Tile",
            Self::CONCRETE_DYNAMIC | Self::CONCRETE_STATIC => "Concrete",
            Self::FLESH_DYNAMIC => "Flesh",
            Self::GRAVEL_DYNAMIC | Self::GRAVEL_STATIC => "Gravel",
            Self::METAL_DYNAMIC | Self::METAL_STATIC | Self::METAL_SLIP => "Metal",
            Self::WOOD_DYNAMIC | Self::WOOD_STATIC => "Wood",
            Self::FOLIAGE_STATIC | Self::FOLIAGE_DYNAMIC => "Foliage",
            Self::SNOW_STATIC => "Snow",
            Self::ICE_STATIC => "Ice",
            Self::WATER_STATIC => "Water",
            Self::ALIEN_DYNAMIC => "Alien",
            Self::SAND_STATIC => "Sand",
            _ => return None,
        })
    }
}

pinewatch_mono::net_enum! {
    /// `[Flags] ERayMask` (`Utils/RayMasks.cs`): `1 << ELayerMask.X` for each Unity layer. `GROUND2`, layer 31, is
    /// `int.MinValue`.
    pub struct ERayMask(i32) {
        DEFAULT = 1 << 0,
        TRANSPARENT_FX = 1 << 1,
        IGNORE_RAYCAST = 1 << 2,
        BUILTIN_3 = 1 << 3,
        WATER = 1 << 4,
        UI = 1 << 5,
        BUILTIN_6 = 1 << 6,
        BUILTIN_7 = 1 << 7,
        LOGIC = 1 << 8,
        PLAYER = 1 << 9,
        ENEMY = 1 << 10,
        VIEWMODEL = 1 << 11,
        DEBRIS = 1 << 12,
        ITEM = 1 << 13,
        RESOURCE = 1 << 14,
        LARGE = 1 << 15,
        MEDIUM = 1 << 16,
        SMALL = 1 << 17,
        SKY = 1 << 18,
        ENVIRONMENT = 1 << 19,
        GROUND = 1 << 20,
        CLIP = 1 << 21,
        NAVMESH = 1 << 22,
        ENTITY = 1 << 23,
        AGENT = 1 << 24,
        LADDER = 1 << 25,
        VEHICLE = 1 << 26,
        BARRICADE = 1 << 27,
        STRUCTURE = 1 << 28,
        TIRE = 1 << 29,
        TRAP = 1 << 30,
        GROUND2 = 1 << 31,
    }
}

/// The fields `LandscapeMaterialAsset.PopulateAsset` reads.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LandscapeMaterialAsset {
    pub texture: ContentReference,
    pub mask: ContentReference,
    pub physics_material: EPhysicsMaterial,
    pub physics_material_name: Option<NetString>,
    pub foliage: AssetReference,
    pub christmas_redirect: AssetReference,
    pub halloween_redirect: AssetReference,
    pub april_fools_redirect: AssetReference,
    pub use_auto_slope: bool,
    pub auto_min_angle_begin: f32,
    pub auto_min_angle_end: f32,
    pub auto_max_angle_begin: f32,
    pub auto_max_angle_end: f32,
    pub use_auto_foundation: bool,
    pub auto_ray_radius: f32,
    pub auto_ray_length: f32,
    pub auto_ray_mask: ERayMask,
}

impl LandscapeMaterialAsset {
    /// `PopulateAsset` after `Asset`'s part. `current_master_bundle` is `Assets.currentMasterBundle`'s bundle
    /// name, the default for content paths without one.
    pub fn populate(data: &DictRef<'_>, current_master_bundle: Option<String>) -> Result<Self, MonoException> {
        let mut context = current_master_bundle;
        let texture = data.parse_struct("Texture", ContentReference::default(), &mut context)?;
        let mask = data.parse_struct("Mask", ContentReference::default(), &mut context)?;
        let mut physics_material_name = data.get_string("Physics_Material", None).cloned();
        // `Enum.TryParse(name, out physicsMaterial)`: case-sensitive, and `default` when it fails.
        let physics_material = try_parse_enum(&EPhysicsMaterial::INFO, physics_material_name.as_ref(), false)
            .map(EPhysicsMaterial::from_bits);
        if let Some(material) = physics_material {
            // Input may have odd case (e.g., all uppercase) which older name matching doesn't handle.
            physics_material_name = material.legacy_name().map(NetString::from);
        }
        let reference = |key: &str| data.parse_struct(key, AssetReference::default(), &mut ());
        Ok(Self {
            texture,
            mask,
            physics_material: physics_material.unwrap_or_default(),
            physics_material_name,
            foliage: reference("Foliage")?,
            christmas_redirect: reference("Christmas_Redirect")?,
            halloween_redirect: reference("Halloween_Redirect")?,
            april_fools_redirect: reference("AprilFools_Redirect")?,
            use_auto_slope: data.parse_bool("Use_Auto_Slope", false),
            auto_min_angle_begin: data.parse_float("Auto_Min_Angle_Begin", 0.0),
            auto_min_angle_end: data.parse_float("Auto_Min_Angle_End", 0.0),
            auto_max_angle_begin: data.parse_float("Auto_Max_Angle_Begin", 0.0),
            auto_max_angle_end: data.parse_float("Auto_Max_Angle_End", 0.0),
            use_auto_foundation: data.parse_bool("Use_Auto_Foundation", false),
            auto_ray_radius: data.parse_float("Auto_Ray_Radius", 0.0),
            auto_ray_length: data.parse_float("Auto_Ray_Length", 0.0),
            auto_ray_mask: data.parse_enum("Auto_Ray_Mask", ERayMask::default()),
        })
    }

    /// `getHolidayRedirect`, given `HolidayUtil.getActiveHoliday()`: the redirect for Christmas, Halloween or
    /// April Fools, else `AssetReference.invalid`.
    pub fn holiday_redirect(&self, active_holiday: ENPCHoliday) -> AssetReference {
        match active_holiday {
            ENPCHoliday::CHRISTMAS => self.christmas_redirect,
            ENPCHoliday::HALLOWEEN => self.halloween_redirect,
            ENPCHoliday::APRIL_FOOLS => self.april_fools_redirect,
            _ => AssetReference::default(),
        }
    }
}
