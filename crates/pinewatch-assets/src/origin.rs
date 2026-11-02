//! `AssetOrigin` (`Bundles/AssetOrigin.cs`) and `Assets.assetOrigins`, the list every origin is added to.

/// Where a group of assets came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetOrigin {
    /// Hard-coded built-in name, or the name of the workshop file.
    pub name: String,
    /// The Steam file ID for workshop content, zero otherwise.
    pub workshop_file_id: u64,
    /// New assets replace existing ones with the same ID or GUID, without warnings.
    pub should_assets_override_existing_ids: bool,
    /// The game may re-save this origin's `.dat` and `.asset` files (only with `-ResaveAssets`, which is off).
    pub can_resave: bool,
}

impl AssetOrigin {
    fn new(name: impl Into<String>) -> Self {
        Self { name: name.into(), workshop_file_id: 0, should_assets_override_existing_ids: false, can_resave: false }
    }
}

/// An origin's place in [`Origins`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OriginId(usize);

/// `Assets.assetOrigins`, with the built-in origins `LoadAllAssets` creates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Origins {
    list: Vec<AssetOrigin>,
}

impl Default for Origins {
    fn default() -> Self {
        Self::new()
    }
}

impl Origins {
    /// The four origins `LoadAllAssets` adds first, in its order. `coreOrigin.canResave` is
    /// `Application.isEditor`, false in the game.
    pub fn new() -> Self {
        let core = AssetOrigin::new("Vanilla Built-in Assets");
        let reload =
            AssetOrigin { should_assets_override_existing_ids: true, ..AssetOrigin::new("Reloaded Assets (Debug)") };
        let shared = AssetOrigin::new("Server Common (Legacy)");
        let per_server = AssetOrigin::new("Per-Server (Legacy)");
        Self { list: vec![core, reload, shared, per_server] }
    }

    /// `Assets.coreOrigin`.
    pub fn core(&self) -> OriginId {
        OriginId(0)
    }

    /// `Assets.reloadOrigin`.
    pub fn reload(&self) -> OriginId {
        OriginId(1)
    }

    pub fn get(&self, id: OriginId) -> &AssetOrigin {
        &self.list[id.0]
    }

    pub fn iter(&self) -> impl Iterator<Item = &AssetOrigin> {
        self.list.iter()
    }

    /// `assetOrigins.Add(origin)`.
    pub(crate) fn add(&mut self, origin: AssetOrigin) -> OriginId {
        self.list.push(origin);
        OriginId(self.list.len() - 1)
    }

    /// The origin `AddSandboxSearchLocations` creates for a folder of `Sandbox`.
    pub(crate) fn add_sandbox(&mut self, folder_name: &str) -> OriginId {
        self.add(AssetOrigin {
            should_assets_override_existing_ids: true,
            can_resave: true,
            ..AssetOrigin::new(format!("Sandbox Folder \"{folder_name}\""))
        })
    }

    /// `FindOrAddLevelOrigin` for a level directly in the `Maps` folder (no workshop file): the first origin named
    /// `Map "<name>"` (ordinal comparison), else a new one that can resave.
    pub(crate) fn find_or_add_level(&mut self, level_name: &str) -> OriginId {
        let expected = format!("Map \"{level_name}\"");
        if let Some(index) = self.list.iter().position(|origin| origin.name == expected) {
            return OriginId(index);
        }
        self.add(AssetOrigin { can_resave: true, ..AssetOrigin::new(expected) })
    }
}
