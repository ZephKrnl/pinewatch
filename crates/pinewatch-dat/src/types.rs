//! The type lookup behind `ParseType` (UnturnedDat/DatValueEx.cs, DatValue.cs).

use pinewatch_mono::{MonoException, NetString};

/// A type the resolver found. What it stands for is up to the resolver.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct TypeHandle(pub u32);

/// `System.Type.GetType(name, throwOnError: false, ignoreCase: true)`: finds a type by name, the way the game's own
/// types and Mono's would be found (`docs/VERIFICATION.md`, V19). `throwOnError: false` returns null for a name it
/// doesn't find, but it can still throw for a malformed or assembly-qualified name, so `resolve` returns the error
/// that `ParseType` propagates.
pub trait TypeResolver {
    fn resolve(&self, assembly_qualified_name: &NetString) -> Result<Option<TypeHandle>, MonoException>;
}

/// `DatValue.INVALID_TYPE_CHARS`: a name containing one of these is never looked up.
pub(crate) const INVALID_TYPE_CHARS: [u16; 3] = [b'\\' as u16, b':' as u16, b'/' as u16];
