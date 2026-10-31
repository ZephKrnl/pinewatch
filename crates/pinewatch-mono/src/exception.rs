use std::fmt;

/// The .NET exception types that ported code can raise.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExceptionKind {
    Argument,
    ArgumentNull,
    ArgumentOutOfRange,
    Format,
    IndexOutOfRange,
    InvalidCast,
    Io,
    NotImplemented,
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
            Self::Format => "System.FormatException",
            Self::IndexOutOfRange => "System.IndexOutOfRangeException",
            Self::InvalidCast => "System.InvalidCastException",
            Self::Io => "System.IO.IOException",
            Self::NotImplemented => "System.NotImplementedException",
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

/// A .NET exception: its type and its exact message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonoException {
    pub kind: ExceptionKind,
    pub message: String,
}

impl MonoException {
    pub fn new(kind: ExceptionKind, message: impl Into<String>) -> Self {
        Self { kind, message: message.into() }
    }

    pub fn format(message: impl Into<String>) -> Self {
        Self::new(ExceptionKind::Format, message)
    }

    /// The Mono runtime's message for dereferencing null.
    pub fn null_reference() -> Self {
        Self::new(ExceptionKind::NullReference, "Object reference not set to an instance of an object")
    }

    /// The Mono runtime's message for a failed cast or unboxing.
    pub fn invalid_cast() -> Self {
        Self::new(ExceptionKind::InvalidCast, "Specified cast is not valid.")
    }

    /// `new OverflowException()`.
    pub fn overflow() -> Self {
        Self::new(ExceptionKind::Overflow, "Arithmetic operation resulted in an overflow.")
    }

    /// What a C# string or array indexer throws when out of range.
    pub fn index_out_of_range() -> Self {
        Self::new(ExceptionKind::IndexOutOfRange, "Index was outside the bounds of the array.")
    }

    /// `new ArgumentOutOfRangeException(paramName)`.
    pub fn argument_out_of_range(param_name: &str) -> Self {
        Self::new(
            ExceptionKind::ArgumentOutOfRange,
            format!("Specified argument was out of the range of valid values.\nParameter name: {param_name}"),
        )
    }

    /// `new ArgumentNullException(paramName)`.
    pub fn argument_null(param_name: &str) -> Self {
        Self::new(ExceptionKind::ArgumentNull, format!("Value cannot be null.\nParameter name: {param_name}"))
    }
}

impl fmt::Display for MonoException {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.kind.type_name(), self.message)
    }
}

impl std::error::Error for MonoException {}
