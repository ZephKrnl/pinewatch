//! `LandscapeMaterialAsset`'s `PopulateAsset` and its holiday redirects (M2c task 11).

use pinewatch_assets::{
    AssetReference, ENPCHoliday, EPhysicsMaterial, ERayMask, LandscapeMaterialAsset, MonoPath, Origins, SearchLocation,
    AssetLoader, read_location,
};
use pinewatch_dat::{DatParser, DatTree};
use pinewatch_io::{InstallRoot, MemoryLog};
use pinewatch_mono::{Guid, NetString};

const G1: &str = "1b2a3d4c5e6f708192a3b4c5d6e7f809";
const G2: &str = "2b2a3d4c5e6f708192a3b4c5d6e7f809";
const G3: &str = "3b2a3d4c5e6f708192a3b4c5d6e7f809";
const G4: &str = "4b2a3d4c5e6f708192a3b4c5d6e7f809";

fn tree(text: &str) -> DatTree {
    DatParser::new().parse_str(text)
}

fn guid(text: &str) -> Guid {
    Guid::try_parse(Some(&NetString::from(text))).unwrap()
}

fn text(value: &Option<NetString>) -> Option<String> {
    value.as_ref().map(NetString::to_string_lossy)
}

fn populate(data: &str, master_bundle: Option<&str>) -> LandscapeMaterialAsset {
    let tree = tree(data);
    LandscapeMaterialAsset::populate(&tree.root(), master_bundle.map(str::to_string)).unwrap()
}

#[test]
fn every_field() {
    let material = populate(
        &format!(
            "Texture other.masterbundle:Materials/Grass/Texture.png\nMask Materials/Grass/Mask.png\n\
             Physics_Material CONCRETE_STATIC\nFoliage {G1}\nChristmas_Redirect {G2}\nHalloween_Redirect {G3}\n\
             AprilFools_Redirect {G4}\nUse_Auto_Slope\nAuto_Min_Angle_Begin 12.5\nAuto_Min_Angle_End 20\n\
             Auto_Max_Angle_Begin 60\nAuto_Max_Angle_End 70.25\nUse_Auto_Foundation true\nAuto_Ray_Radius 1.5\n\
             Auto_Ray_Length 8\nAuto_Ray_Mask \"GROUND, LARGE\""
        ),
        Some("core.masterbundle"),
    );
    assert_eq!(text(&material.texture.name).as_deref(), Some("other.masterbundle"));
    assert_eq!(text(&material.texture.path).as_deref(), Some("Materials/Grass/Texture.png"));
    assert_eq!(text(&material.mask.name).as_deref(), Some("core.masterbundle"));
    assert_eq!(text(&material.mask.path).as_deref(), Some("Materials/Grass/Mask.png"));
    assert_eq!(material.physics_material, EPhysicsMaterial::CONCRETE_STATIC);
    assert_eq!(text(&material.physics_material_name).as_deref(), Some("Concrete"));
    assert_eq!(material.foliage, AssetReference { guid: guid(G1) });
    assert_eq!(material.christmas_redirect, AssetReference { guid: guid(G2) });
    assert_eq!(material.halloween_redirect, AssetReference { guid: guid(G3) });
    assert_eq!(material.april_fools_redirect, AssetReference { guid: guid(G4) });
    // `ParseBool` on a flag is false: a flag's value is null.
    assert!(!material.use_auto_slope);
    assert!(material.use_auto_foundation);
    let angles = [
        material.auto_min_angle_begin,
        material.auto_min_angle_end,
        material.auto_max_angle_begin,
        material.auto_max_angle_end,
    ];
    assert_eq!(angles, [12.5, 20.0, 60.0, 70.25]);
    assert_eq!((material.auto_ray_radius, material.auto_ray_length), (1.5, 8.0));
    assert_eq!(material.auto_ray_mask, ERayMask(ERayMask::GROUND.0 | ERayMask::LARGE.0));
    assert_eq!(material.auto_ray_mask, ERayMask((1 << 20) | (1 << 15)));
}

/// `ParseEnum` ignores case and takes numbers; `GROUND2` is bit 31, `int.MinValue`; anything else is the default.
#[test]
fn ray_masks() {
    let cases = [
        ("Auto_Ray_Mask ground", 1 << 20),
        ("Auto_Ray_Mask GROUND2", i32::MIN),
        ("Auto_Ray_Mask \"GROUND2, DEFAULT\"", i32::MIN | 1),
        ("Auto_Ray_Mask 12", 12),
        ("Auto_Ray_Mask -1", -1),
        ("Auto_Ray_Mask nonsense", 0),
        ("Auto_Ray_Mask \"GROUND, nonsense\"", 0),
    ];
    for (data, mask) in cases {
        assert_eq!(populate(data, None).auto_ray_mask, ERayMask(mask), "{data}");
    }
}

/// `Enum.TryParse(name, out physicsMaterial)` is case-sensitive; a legacy name becomes the new name, a value no
/// member maps to (or `NONE`) becomes null, and anything else stays as written.
#[test]
fn physics_material_names() {
    let cases = [
        ("Physics_Material CONCRETE_STATIC", 6, Some("Concrete")),
        ("Physics_Material METAL_SLIP", 12, Some("Metal")),
        ("Physics_Material Grass", 0, Some("Grass")),
        ("Physics_Material concrete_static", 0, Some("concrete_static")),
        ("Physics_Material NONE", 0, None),
        ("Physics_Material 99", 99, None),
        ("", 0, None),
    ];
    for (data, material, name) in cases {
        let populated = populate(data, None);
        assert_eq!(populated.physics_material, EPhysicsMaterial(material), "{data}");
        assert_eq!(text(&populated.physics_material_name).as_deref(), name, "{data}");
    }
}

#[test]
fn defaults_without_data() {
    let material = populate("", None);
    assert_eq!(material.texture.name, None);
    assert!(!material.texture.is_valid() && !material.foliage.is_valid());
    assert_eq!(material.auto_ray_mask, ERayMask(0));
    assert_eq!(material.auto_ray_radius, 0.0);
}

#[test]
fn holiday_redirects() {
    let data = format!("Christmas_Redirect {G2}\nHalloween_Redirect {G3}\nAprilFools_Redirect {G4}");
    let material = populate(&data, None);
    assert_eq!(material.holiday_redirect(ENPCHoliday::CHRISTMAS).guid, guid(G2));
    assert_eq!(material.holiday_redirect(ENPCHoliday::HALLOWEEN).guid, guid(G3));
    assert_eq!(material.holiday_redirect(ENPCHoliday::APRIL_FOOLS).guid, guid(G4));
    // The server sends the holiday, so a value no member has can arrive; it takes the `default` case.
    let others = [
        ENPCHoliday::NONE,
        ENPCHoliday::VALENTINES,
        ENPCHoliday::PRIDE_MONTH,
        ENPCHoliday::LUNAR_NEW_YEAR,
        ENPCHoliday::UNTURNED_ANNIVERSARY,
        ENPCHoliday::MAX,
        ENPCHoliday(42),
        ENPCHoliday(-1),
    ];
    for holiday in others {
        assert!(!material.holiday_redirect(holiday).is_valid(), "{holiday:?}");
    }
}

/// The loader populates landscape materials, with the master bundle they load from as the content default.
#[test]
fn loaded_through_the_asset_loader() {
    let root = std::env::temp_dir().join(format!("pinewatch-assets-material-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let core_config = "Asset_Bundle_Name core.masterbundle\nAsset_Prefix Assets/Bundles\nMaster_Bundle_Version 6";
    let files = [
        ("Bundles/MasterBundle.dat", core_config.to_string()),
        ("Bundles/core.masterbundle", String::new()),
        (
            "Bundles/Materials/Grass_Material/Grass_Material.asset",
            format!(
                "Metadata\n{{\n\tGUID {G1}\n\tType \"SDG.Unturned.LandscapeMaterialAsset, Assembly-CSharp\"\n}}\n\
                 Asset\n{{\n\tTexture Texture.png\n\tPhysics_Material GRAVEL_STATIC\n}}"
            ),
        ),
    ];
    for (file, contents) in &files {
        let path = root.join(file);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, contents).unwrap();
    }
    let install = InstallRoot::open(&root).unwrap();
    let origins = Origins::new();
    let separator = if cfg!(windows) { "\\" } else { "/" };
    let bundles = MonoPath { full: format!("{}{separator}Bundles", install.full_name()), rel: "Bundles".to_string() };
    let location = SearchLocation { dir: bundles, origin: origins.core() };
    let mut loader = AssetLoader::new(origins);
    let mut log = MemoryLog::default();
    loader.take_results(read_location(&install, &location, "English").unwrap(), &mut log);
    let _ = std::fs::remove_dir_all(&root);

    let header = &loader.headers[0];
    let material = header.landscape_material.as_ref().expect("a populated landscape material");
    assert_eq!(text(&material.texture.name).as_deref(), Some("core.masterbundle"));
    assert_eq!(text(&material.texture.path).as_deref(), Some("Texture.png"));
    assert_eq!(text(&material.physics_material_name).as_deref(), Some("Gravel"));
    assert_eq!(header.friendly_name, "Grass");
}
