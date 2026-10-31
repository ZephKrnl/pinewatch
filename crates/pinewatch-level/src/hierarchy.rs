//! The item loop of `LevelHierarchy.read` (Framework/Devkit/LevelHierarchy.cs).

use pinewatch_io::{LogLevel, LogSink};
use pinewatch_kvt::{KvtCursor, KvtTable};
use pinewatch_mono::{MonoException, NetString};

/// One entry of `Level.hierarchy`'s `Items` list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HierarchyItem {
    pub index: i32,
    /// The assembly-qualified type name after redirects, or `None` when the original logs "missing type".
    pub type_name: Option<NetString>,
    /// The item's instance ID, generated from `Available_Instance_ID` when missing or 0.
    pub instance_id: Option<u32>,
}

/// Reads every item, logging what the original logs for items without a usable type.
pub fn read_hierarchy_items(table: &KvtTable, log: &mut dyn LogSink) -> Result<Vec<HierarchyItem>, MonoException> {
    walk_hierarchy(table, log, |_, _, _| Ok(()))
}

/// `LevelHierarchy.read`. `visit` stands in for `item.read(itemReader)`: it receives each item with a type, and the
/// item's reader positioned on the `Item` key, in file order.
///
/// Resolving the type name (`Type.GetType`) and creating the C# object are not ported yet: every non-empty valid
/// name is treated as a type that exists (`docs/VERIFICATION.md`, V10).
pub(crate) fn walk_hierarchy<F>(
    table: &KvtTable,
    log: &mut dyn LogSink,
    mut visit: F,
) -> Result<Vec<HierarchyItem>, MonoException>
where
    F: FnMut(&HierarchyItem, KvtCursor<'_>, &mut dyn LogSink) -> Result<(), MonoException>,
{
    let mut reader = KvtCursor::new(table);

    let mut available_instance_id: u32 = if reader.contains_key("Available_Instance_ID") {
        reader.read_key("Available_Instance_ID");
        reader.read_u32()?
    } else {
        1
    };

    let items_count = reader.read_array_length("Items")?;
    let mut items = Vec::new();
    for item_index in 0..items_count {
        let mut item_reader = reader.read_object_index(item_index)?.ok_or_else(MonoException::null_reference)?;

        item_reader.read_key("Type");
        let Some(type_name) = item_reader.read_type_name()? else {
            let raw = item_reader.read_value_key("Type")?.map(|s| s.to_string_lossy()).unwrap_or_default();
            log.log(LogLevel::Error, &format!("Level hierarchy item index {item_index} missing type: {raw}"));
            items.push(HierarchyItem { index: item_index, type_name: None, instance_id: None });
            continue;
        };

        let mut instance_id = 0u32;
        if item_reader.contains_key("Instance_ID") {
            item_reader.read_key("Instance_ID");
            instance_id = item_reader.read_u32()?;
        }
        if instance_id == 0 {
            // generateUniqueInstanceID
            instance_id = available_instance_id;
            available_instance_id = available_instance_id.wrapping_add(1);
        }

        let item = HierarchyItem { index: item_index, type_name: Some(type_name), instance_id: Some(instance_id) };
        item_reader.read_key("Item");
        visit(&item, item_reader, log)?;
        items.push(item);
    }
    Ok(items)
}
