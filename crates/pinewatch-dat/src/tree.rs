//! The parsed node tree: `DatDictionary`, `DatList` and `DatValue` (UnturnedDat/DatDictionary.cs, DatList.cs,
//! DatValue.cs, DatNode.cs), and in metadata mode their `…WithMetadata` wrappers (DatNodeWithMetadata.cs,
//! DatDictionaryWithMetadata.cs, DatListWithMetadata.cs, DatValueWithMetadata.cs). All nodes of one parse live in
//! one arena and refer to each other by index.

use std::collections::HashMap;
use std::collections::hash_map::Entry;

use pinewatch_mono::{NetString, chars};

use crate::comment::DatComment;

/// `EDatNodeType`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DatNodeType {
    Value,
    Dictionary,
    List,
}

/// A node's place in its tree.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NodeId(usize);

/// The root is the first node of every tree.
const ROOT: NodeId = NodeId(0);

/// What `DatParser.Parse` returns: the root dictionary and every node under it.
#[derive(Debug)]
pub struct DatTree {
    nodes: Vec<Node>,
}

#[derive(Debug)]
struct Node {
    data: NodeData,
    /// Present when the node was parsed with metadata on: the node is then a `…WithMetadata` wrapper.
    metadata: Option<Metadata>,
}

#[derive(Debug)]
enum NodeData {
    /// `DatValue.value`, which can be null.
    Value(Option<NetString>),
    Dictionary(Dictionary),
    /// `DatList`'s items, in order.
    List(Vec<NodeId>),
}

/// The fields of the `…WithMetadata` wrappers.
#[derive(Debug, Default)]
pub(crate) struct Metadata {
    /// `parentNode`: `None` for the root.
    pub(crate) parent: Option<NodeId>,
    /// `prefixComment`.
    pub(crate) prefix_comment: Option<DatComment>,
    /// A value's `lineNumber`, or a dictionary's or list's `openingLineNumber`.
    pub(crate) line_number: i32,
    /// A dictionary's or list's `closingLineNumber`.
    pub(crate) closing_line_number: i32,
    /// A value's `inlineComment`.
    pub(crate) inline_comment: Option<NetString>,
}

/// `DatDictionary`: a `Dictionary<string, IDatNode>` with `StringComparer.OrdinalIgnoreCase`.
#[derive(Debug, Default)]
struct Dictionary {
    /// The pairs in the order the C# dictionary enumerates them. Nothing is ever removed, so that's insertion order,
    /// and a key whose value is replaced keeps its place and its first spelling.
    entries: Vec<(NetString, NodeId)>,
    /// Each key with every code unit through `ToUpperInvariant`, to its place in `entries`. Two keys are equal under
    /// `OrdinalIgnoreCase` exactly when these match.
    lookup: HashMap<Vec<u16>, usize>,
}

impl Dictionary {
    /// `TryGetValue(key, out node)`.
    fn find(&self, key: &[u16]) -> Option<NodeId> {
        self.lookup.get(&fold_case(key)).map(|&index| self.entries[index].1)
    }

    /// `this[key] = node`: replaces the value of an existing key, or adds the pair at the end.
    fn set(&mut self, key: NetString, node: NodeId) {
        match self.lookup.entry(fold_case(key.as_units())) {
            Entry::Occupied(entry) => self.entries[*entry.get()].1 = node,
            Entry::Vacant(entry) => {
                entry.insert(self.entries.len());
                self.entries.push((key, node));
            }
        }
    }
}

/// What `OrdinalIgnoreCase` compares: every code unit through `ToUpperInvariant`.
fn fold_case(key: &[u16]) -> Vec<u16> {
    key.iter().map(|&unit| chars::to_upper_invariant(unit)).collect()
}

impl DatTree {
    /// A tree holding only an empty root dictionary.
    pub(crate) fn new() -> Self {
        Self { nodes: vec![Node { data: NodeData::Dictionary(Dictionary::default()), metadata: None }] }
    }

    /// The root dictionary.
    pub fn root(&self) -> DictRef<'_> {
        let NodeData::Dictionary(dictionary) = &self.nodes[ROOT.0].data else {
            unreachable!("the first node is always the root dictionary");
        };
        DictRef { tree: self, id: ROOT, dictionary }
    }

    pub(crate) fn root_id(&self) -> NodeId {
        ROOT
    }

    /// `new DatValue(value)`.
    pub(crate) fn add_value(&mut self, value: Option<NetString>) -> NodeId {
        self.add(NodeData::Value(value))
    }

    /// `new DatDictionary()`.
    pub(crate) fn add_dictionary(&mut self) -> NodeId {
        self.add(NodeData::Dictionary(Dictionary::default()))
    }

    /// `new DatList()`.
    pub(crate) fn add_list(&mut self) -> NodeId {
        self.add(NodeData::List(Vec::new()))
    }

    fn add(&mut self, data: NodeData) -> NodeId {
        self.nodes.push(Node { data, metadata: None });
        NodeId(self.nodes.len() - 1)
    }

    /// Wraps the node in its `…WithMetadata` class, with every field at its default.
    pub(crate) fn add_metadata(&mut self, id: NodeId) -> &mut Metadata {
        self.nodes[id.0].metadata.get_or_insert_with(Metadata::default)
    }

    /// The fields of the node's `…WithMetadata` wrapper, if it has one.
    pub(crate) fn metadata_mut(&mut self, id: NodeId) -> Option<&mut Metadata> {
        self.nodes[id.0].metadata.as_mut()
    }

    /// `dictionary.TryGetNode(key, out node)`.
    pub(crate) fn dictionary_try_get_node(&self, dictionary: NodeId, key: &[u16]) -> Option<NodeId> {
        match &self.nodes[dictionary.0].data {
            NodeData::Dictionary(dictionary) => dictionary.find(key),
            _ => None,
        }
    }

    /// `dictionary[key] = node`.
    pub(crate) fn dictionary_set(&mut self, dictionary: NodeId, key: NetString, node: NodeId) {
        if let NodeData::Dictionary(dictionary) = &mut self.nodes[dictionary.0].data {
            dictionary.set(key, node);
        }
    }

    /// `list.Add(node)`.
    pub(crate) fn list_add(&mut self, list: NodeId, node: NodeId) {
        if let NodeData::List(items) = &mut self.nodes[list.0].data {
            items.push(node);
        }
    }

    /// The node with this id.
    pub(crate) fn node(&self, id: NodeId) -> NodeRef<'_> {
        match &self.nodes[id.0].data {
            NodeData::Value(value) => NodeRef::Value(ValueRef { tree: self, id, value }),
            NodeData::Dictionary(dictionary) => NodeRef::Dictionary(DictRef { tree: self, id, dictionary }),
            NodeData::List(items) => NodeRef::List(ListRef { tree: self, id, items }),
        }
    }

    fn metadata(&self, id: NodeId) -> Option<&Metadata> {
        self.nodes[id.0].metadata.as_ref()
    }
}

/// `IDatNode`: any node of a tree.
#[derive(Clone, Copy)]
pub enum NodeRef<'t> {
    Value(ValueRef<'t>),
    Dictionary(DictRef<'t>),
    List(ListRef<'t>),
}

impl<'t> From<ValueRef<'t>> for NodeRef<'t> {
    fn from(value: ValueRef<'t>) -> Self {
        Self::Value(value)
    }
}

impl<'t> From<DictRef<'t>> for NodeRef<'t> {
    fn from(dictionary: DictRef<'t>) -> Self {
        Self::Dictionary(dictionary)
    }
}

impl<'t> From<ListRef<'t>> for NodeRef<'t> {
    fn from(list: ListRef<'t>) -> Self {
        Self::List(list)
    }
}

impl<'t> NodeRef<'t> {
    /// `NodeType`.
    pub fn node_type(&self) -> DatNodeType {
        match self {
            Self::Value(_) => DatNodeType::Value,
            Self::Dictionary(_) => DatNodeType::Dictionary,
            Self::List(_) => DatNodeType::List,
        }
    }

    /// `node as IDatValue`.
    pub fn as_value(&self) -> Option<ValueRef<'t>> {
        if let Self::Value(value) = self { Some(*value) } else { None }
    }

    /// `node as IDatDictionary`.
    pub fn as_dictionary(&self) -> Option<DictRef<'t>> {
        if let Self::Dictionary(dictionary) = self { Some(*dictionary) } else { None }
    }

    /// `node as IDatList`.
    pub fn as_list(&self) -> Option<ListRef<'t>> {
        if let Self::List(list) = self { Some(*list) } else { None }
    }

    /// The node's place in its tree.
    pub fn id(&self) -> NodeId {
        self.location().1
    }

    /// `IsMetadataAvailable`: whether the node was parsed with metadata on.
    pub fn is_metadata_available(&self) -> bool {
        self.metadata().is_some()
    }

    /// `TryGetParsedComment`: the comment lines right before the node.
    pub fn parsed_comment(&self) -> Option<&'t DatComment> {
        self.metadata()?.prefix_comment.as_ref()
    }

    /// `TryGetParentNode`: the dictionary or list holding the node. `None` for the root, and for every node parsed
    /// without metadata (`is_metadata_available` tells the two apart).
    pub fn parent(&self) -> Option<NodeRef<'t>> {
        let (tree, _) = self.location();
        self.metadata()?.parent.map(|parent| tree.node(parent))
    }

    /// `TryGetParsedLineNumber`: a value's first line, or the line of a dictionary's `{` or a list's `[`. The root
    /// is line 1. A dictionary or list that was never closed reports 0.
    pub fn parsed_line_number(&self) -> Option<i32> {
        Some(self.metadata()?.line_number)
    }

    /// `TryGetParsedLineNumberRange`: a value's line twice, or the lines of a dictionary's or list's opening and
    /// closing brackets. The root spans the whole file.
    pub fn parsed_line_number_range(&self) -> Option<(i32, i32)> {
        let metadata = self.metadata()?;
        match self {
            Self::Value(_) => Some((metadata.line_number, metadata.line_number)),
            Self::Dictionary(_) | Self::List(_) => Some((metadata.line_number, metadata.closing_line_number)),
        }
    }

    /// `TryGetNodePath`: `/` and the key or index of each step from the root, for example `/Blueprints/0/Name`.
    /// The root's path is empty. `None` without metadata.
    pub fn node_path(&self) -> Option<NetString> {
        let metadata = self.metadata()?;
        let Some(parent) = metadata.parent else {
            return Some(NetString::new());
        };
        let (tree, id) = self.location();
        let parent = tree.node(parent);
        let mut path = parent.node_path()?.into_units();
        match parent {
            NodeRef::Dictionary(dictionary) => {
                let (key, _) = dictionary.iter().find(|(_, node)| node.id() == id)?;
                path.push(u16::from(b'/'));
                path.extend_from_slice(key.as_units());
            }
            NodeRef::List(list) => {
                let index = list.iter().position(|node| node.id() == id)?;
                path.push(u16::from(b'/'));
                path.extend(index.to_string().encode_utf16());
            }
            NodeRef::Value(_) => return None,
        }
        Some(NetString::from_units(path))
    }

    fn location(&self) -> (&'t DatTree, NodeId) {
        match self {
            Self::Value(value) => (value.tree, value.id),
            Self::Dictionary(dictionary) => (dictionary.tree, dictionary.id),
            Self::List(list) => (list.tree, list.id),
        }
    }

    fn metadata(&self) -> Option<&'t Metadata> {
        let (tree, id) = self.location();
        tree.metadata(id)
    }
}

/// `IDatValue`.
#[derive(Clone, Copy)]
pub struct ValueRef<'t> {
    tree: &'t DatTree,
    id: NodeId,
    value: &'t Option<NetString>,
}

impl<'t> ValueRef<'t> {
    /// `Value`: the text, or `None` where the C# has null.
    pub fn value(&self) -> Option<&'t NetString> {
        self.value.as_ref()
    }

    /// `TryGetParsedInlineComment`: the comment after a quoted value on its line, or `None` (null in the C#) when
    /// there is none, when it's empty, or without metadata.
    pub fn parsed_inline_comment(&self) -> Option<&'t NetString> {
        self.tree.metadata(self.id)?.inline_comment.as_ref()
    }

    /// The node's place in its tree.
    pub fn id(&self) -> NodeId {
        self.id
    }
}

/// `IDatDictionary`.
#[derive(Clone, Copy)]
pub struct DictRef<'t> {
    tree: &'t DatTree,
    id: NodeId,
    dictionary: &'t Dictionary,
}

impl<'t> DictRef<'t> {
    /// `Count`.
    pub fn len(&self) -> usize {
        self.dictionary.entries.len()
    }

    /// `Count == 0`.
    pub fn is_empty(&self) -> bool {
        self.dictionary.entries.is_empty()
    }

    /// `ContainsKey`, which ignores case as `StringComparer.OrdinalIgnoreCase` does.
    pub fn contains_key(&self, key: &str) -> bool {
        self.find(key).is_some()
    }

    /// `TryGetNode`: the node under `key`, ignoring case.
    pub fn try_get_node(&self, key: &str) -> Option<NodeRef<'t>> {
        self.find(key).map(|id| self.tree.node(id))
    }

    /// `TryGetKeyLineNumber`. Never available for a parsed dictionary: the metadata wrapper passes the call to the
    /// plain `DatDictionary`, which has no key line numbers.
    pub fn key_line_number(&self, _key: &str) -> Option<i32> {
        None
    }

    /// The pairs as the C# enumerates them: in insertion order, each key as first spelled.
    pub fn iter(&self) -> impl Iterator<Item = (&'t NetString, NodeRef<'t>)> + use<'t> {
        let tree = self.tree;
        self.dictionary.entries.iter().map(move |(key, id)| (key, tree.node(*id)))
    }

    /// The node's place in its tree.
    pub fn id(&self) -> NodeId {
        self.id
    }

    fn find(&self, key: &str) -> Option<NodeId> {
        let key: Vec<u16> = key.encode_utf16().collect();
        self.dictionary.find(&key)
    }
}

/// `IDatList`.
#[derive(Clone, Copy)]
pub struct ListRef<'t> {
    tree: &'t DatTree,
    id: NodeId,
    items: &'t [NodeId],
}

impl<'t> ListRef<'t> {
    /// `Count`.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// `Count == 0`.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// `TryGetNode(index)`: the item at `index`, or `None` outside `0 .. Count`.
    pub fn try_get_node(&self, index: i32) -> Option<NodeRef<'t>> {
        let index = usize::try_from(index).ok()?;
        self.items.get(index).map(|&id| self.tree.node(id))
    }

    /// The items in order.
    pub fn iter(&self) -> impl Iterator<Item = NodeRef<'t>> + use<'t> {
        let tree = self.tree;
        self.items.iter().map(move |&id| tree.node(id))
    }

    /// `GetValues()`: the items that are values, in order.
    pub fn values(&self) -> impl Iterator<Item = ValueRef<'t>> + use<'t> {
        self.iter().filter_map(|node| node.as_value())
    }

    /// The node's place in its tree.
    pub fn id(&self) -> NodeId {
        self.id
    }
}
