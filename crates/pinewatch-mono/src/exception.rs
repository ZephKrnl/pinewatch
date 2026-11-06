// `ArgumentException.Message` and the default messages are ported from corefx `src/Common/src/CoreLib/System/`
// (mono/corefx 5b4edb9b). Copyright (c) .NET Foundation and Contributors. MIT license; see NOTICES-PORTED-CODE.md.

use std::fmt;

use crate::sr;

/// `Environment.NewLine`: Mono's `GetNewLine` icall returns `"\r\n"` on Windows (`mono/metadata/icall-windows.c`)
/// and `"\n"` elsewhere. `ArgumentException.Message` puts it before the `Parameter name:` line.
pub const NEW_LINE: &str = if cfg!(windows) { "\r\n" } else { "\n" };

/// The message of the `NullReferenceException` the runtime raises for dereferencing null: a C string in the game's
/// native runtime (`MonoBleedingEdge/EmbedRuntime/mono-2.0-bdwgc.dll`), not an `mscorlib.dll` resource.
pub const RUNTIME_NULL_REFERENCE: &str = "Object reference not set to an instance of an object";

/// The .NET exception types that ported code can raise.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExceptionKind {
    Argument,
    ArgumentNull,
    ArgumentOutOfRange,
    /// `System.IO.DirectoryNotFoundException`.
    DirectoryNotFound,
    /// `System.Exception` itself, as game code throws it with a message.
    Exception,
    /// `System.IO.FileLoadException`, which `Type.GetType` throws even with `throwOnError: false`.
    FileLoad,
    Format,
    IndexOutOfRange,
    InvalidCast,
    /// `Newtonsoft.Json.JsonReaderException`.
    JsonReader,
    /// `Newtonsoft.Json.JsonSerializationException`.
    JsonSerialization,
    /// `System.Collections.Generic.KeyNotFoundException`.
    KeyNotFound,
    /// `System.MissingMethodException`.
    MissingMethod,
    Io,
    NullReference,
    OutOfMemory,
    Overflow,
}

impl ExceptionKind {
    /// Full .NET type name, as it appears in logs.
    pub fn type_name(self) -> &'static str {
        match self {
            Self::Argument => "System.ArgumentException",
            Self::ArgumentNull => "System.ArgumentNullException",
            Self::ArgumentOutOfRange => "System.ArgumentOutOfRangeException",
            Self::DirectoryNotFound => "System.IO.DirectoryNotFoundException",
            Self::Exception => "System.Exception",
            Self::FileLoad => "System.IO.FileLoadException",
            Self::Format => "System.FormatException",
            Self::IndexOutOfRange => "System.IndexOutOfRangeException",
            Self::InvalidCast => "System.InvalidCastException",
            Self::JsonReader => "Newtonsoft.Json.JsonReaderException",
            Self::JsonSerialization => "Newtonsoft.Json.JsonSerializationException",
            Self::KeyNotFound => "System.Collections.Generic.KeyNotFoundException",
            Self::MissingMethod => "System.MissingMethodException",
            Self::Io => "System.IO.IOException",
            Self::NullReference => "System.NullReferenceException",
            Self::OutOfMemory => "System.OutOfMemoryException",
            Self::Overflow => "System.OverflowException",
        }
    }

    /// True for `ArgumentException` and the types derived from it.
    pub fn is_argument_exception(self) -> bool {
        matches!(self, Self::Argument | Self::ArgumentNull | Self::ArgumentOutOfRange)
    }
}

/// A .NET exception: its type, its exact `Message`, and the exception it wraps (`InnerException`), if any.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonoException {
    pub kind: ExceptionKind,
    pub message: String,
    pub inner: Option<Box<MonoException>>,
}

/// `ArgumentException.Message` (corefx): the message, then `Environment.NewLine` and `Parameter name: <name>` when
/// the parameter name is neither null nor empty.
fn with_param_name(message: &str, param_name: Option<&str>) -> String {
    match param_name {
        Some(name) if !name.is_empty() => {
            format!("{message}{NEW_LINE}{}", sr::ARG_PARAM_NAME_NAME.replace("{0}", name))
        }
        _ => message.to_string(),
    }
}

impl MonoException {
    pub fn new(kind: ExceptionKind, message: impl Into<String>) -> Self {
        Self { kind, message: message.into(), inner: None }
    }

    /// The same exception with `inner` as its `InnerException`.
    pub fn with_inner(mut self, inner: MonoException) -> Self {
        self.inner = Some(Box::new(inner));
        self
    }

    /// `new FormatException(message)`.
    pub fn format(message: impl Into<String>) -> Self {
        Self::new(ExceptionKind::Format, message)
    }

    /// `new OverflowException(message)`.
    pub fn overflow_message(message: impl Into<String>) -> Self {
        Self::new(ExceptionKind::Overflow, message)
    }

    /// `new ArgumentException(message, paramName)`; `None` is the one-argument constructor.
    pub fn argument(message: &str, param_name: Option<&str>) -> Self {
        Self::new(ExceptionKind::Argument, with_param_name(message, param_name))
    }

    /// What the runtime throws for dereferencing null (a call, a field, an array, `throw null`, unboxing null). The
    /// runtime gives it its own text, a C string in the game's `mono-2.0-bdwgc.dll` without the final period of
    /// `new NullReferenceException()`'s resource string ([`sr::ARG_NULL_REFERENCE_EXCEPTION`]).
    pub fn null_reference() -> Self {
        Self::new(ExceptionKind::NullReference, RUNTIME_NULL_REFERENCE)
    }

    /// What the runtime throws for a failed cast or unboxing: `new InvalidCastException()`.
    pub fn invalid_cast() -> Self {
        Self::new(ExceptionKind::InvalidCast, sr::ARG_INVALID_CAST_EXCEPTION)
    }

    /// `new OverflowException()`, which is also what the runtime throws for checked arithmetic and conversions.
    pub fn overflow() -> Self {
        Self::new(ExceptionKind::Overflow, sr::ARG_OVERFLOW_EXCEPTION)
    }

    /// What `new T[n]` throws for a negative `int` size on the game's x64 runtime: an `OverflowException` with an
    /// empty message (the array allocation's own error). Unity's 32-bit Editor runtime gives the default message
    /// instead, so this was checked on the game's runtime (V30).
    pub fn negative_array_size() -> Self {
        Self::new(ExceptionKind::Overflow, "")
    }

    /// What an array indexer, or a span indexer, throws when out of range: `new IndexOutOfRangeException()`.
    pub fn index_out_of_range() -> Self {
        Self::new(ExceptionKind::IndexOutOfRange, sr::ARG_INDEX_OUT_OF_RANGE_EXCEPTION)
    }

    /// `new ArgumentOutOfRangeException()`, as span slicing throws it.
    pub fn argument_out_of_range_default() -> Self {
        Self::new(ExceptionKind::ArgumentOutOfRange, sr::ARG_ARGUMENT_OUT_OF_RANGE_EXCEPTION)
    }

    /// `new ArgumentOutOfRangeException(paramName)`.
    pub fn argument_out_of_range(param_name: &str) -> Self {
        Self::new(
            ExceptionKind::ArgumentOutOfRange,
            with_param_name(sr::ARG_ARGUMENT_OUT_OF_RANGE_EXCEPTION, Some(param_name)),
        )
    }

    /// `new ArgumentOutOfRangeException(paramName, message)`.
    pub fn argument_out_of_range_message(param_name: &str, message: &str) -> Self {
        Self::new(ExceptionKind::ArgumentOutOfRange, with_param_name(message, Some(param_name)))
    }

    /// `ThrowHelper.ThrowArgumentOutOfRange_IndexException()`, which `List<T>`'s indexer throws.
    pub fn list_index_out_of_range() -> Self {
        Self::argument_out_of_range_message("index", sr::ARGUMENT_OUT_OF_RANGE_INDEX)
    }

    /// `new ArgumentNullException(paramName)`.
    pub fn argument_null(param_name: &str) -> Self {
        Self::new(ExceptionKind::ArgumentNull, with_param_name(sr::ARGUMENT_NULL_GENERIC, Some(param_name)))
    }
}

/// `Type: message`, followed by ` ---> ` and the inner exception when there is one. This is Pinewatch's own
/// error text; logged C# exceptions (`Exception.ToString()`) are reproduced where their catch sites are ported.
impl fmt::Display for MonoException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.kind.type_name(), self.message)?;
        if let Some(inner) = &self.inner {
            write!(f, " ---> {inner}")?;
        }
        Ok(())
    }
}

impl std::error::Error for MonoException {}
