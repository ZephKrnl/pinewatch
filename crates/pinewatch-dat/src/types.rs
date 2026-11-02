//! The type lookup behind `ParseType` (UnturnedDat/DatValueEx.cs, DatValue.cs).

use pinewatch_mono::{MonoException, NetString};

/// A type the resolver found. What it stands for is up to the resolver.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct TypeHandle(pub u32);

/// `System.Type.GetType(name, throwOnError: false, ignoreCase: true)`: finds a type by name, the way the game's own
/// types and Mono's would be found (`docs/VERIFICATION.md`, V19). `Ok(None)` is a type that wasn't found. `Err` is an
/// exception `Type.GetType` throws despite `throwOnError: false`, such as `FileLoadException` for an assembly that is
/// found but can't be loaded; `ParseType` doesn't catch it.
pub trait TypeResolver {
    fn resolve(&self, assembly_qualified_name: &NetString) -> Result<Option<TypeHandle>, MonoException>;
}

/// `DatValue.INVALID_TYPE_CHARS`: a name containing one of these is never looked up.
pub(crate) const INVALID_TYPE_CHARS: [u16; 3] = [b'\\' as u16, b':' as u16, b'/' as u16];
