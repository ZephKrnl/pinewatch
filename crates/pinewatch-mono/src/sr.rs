// Resource strings from corefx (Copyright (c) .NET Foundation and Contributors) and the .NET Framework reference
// source (Copyright (c) Microsoft Corporation), as Unity's Mono carries them. MIT license; see NOTICES-PORTED-CODE.md.

//! The class-library resource strings that ported code puts in exception messages, in English.
//!
//! Unity's Mono build compiles the English text into every call site, including the ones whose source passes a
//! resource key (`SR.GetResourceString(nameof(SR.Format_GuidInvLen))` in corefx, `Environment.GetResourceString(
//! "Argument_InvalidNumberStyles")` in reference source). Both lookup methods just return their argument, so the
//! message is always the English text (checked in the shipped `mscorlib.dll`'s IL; see `docs/VERIFICATION.md`, V0).
//!
//! Each constant is named after its resource key. The texts come from `mcs/class/corlib/corefx/SR.cs` and
//! `mcs/class/corlib/ReferenceSources/SR.cs` in Unity's Mono. The install test `shipped_mscorlib` checks that each
//! one is a string literal in the shipped `mscorlib.dll`.

/// `SR.Argument_AddingDuplicateWithKey`, from `Dictionary.Add`.
pub const ARGUMENT_ADDING_DUPLICATE_WITH_KEY: &str = "An item with the same key has already been added. Key: {0}";
pub const ARG_ARGUMENT_OUT_OF_RANGE_EXCEPTION: &str = "Specified argument was out of the range of valid values.";
pub const ARG_CANNOT_HAVE_NEGATIVE_VALUE: &str = "String cannot contain a minus sign if the base is not 10.";
pub const ARG_HEX_STYLE_NOT_SUPPORTED: &str =
    "The number style AllowHexSpecifier is not supported on floating point data types.";
pub const ARG_INDEX_OUT_OF_RANGE_EXCEPTION: &str = "Index was outside the bounds of the array.";
pub const ARG_INVALID_BASE: &str = "Invalid Base.";
pub const ARG_INVALID_CAST_EXCEPTION: &str = "Specified cast is not valid.";
pub const ARG_INVALID_HEX_STYLE: &str = "With the AllowHexSpecifier bit set in the enum bit field, the only other valid bits that can be combined into the enum value must be a subset of those in HexNumber.";
pub const ARG_NULL_REFERENCE_EXCEPTION: &str = "Object reference not set to an instance of an object.";
pub const ARG_OVERFLOW_EXCEPTION: &str = "Arithmetic operation resulted in an overflow.";
/// `ArgumentException.Message` formats this with the parameter name.
pub const ARG_PARAM_NAME_NAME: &str = "Parameter name: {0}";
pub const ARGUMENT_INVALID_NUMBER_STYLES: &str = "An undefined NumberStyles value is being used.";
pub const ARGUMENT_NULL_GENERIC: &str = "Value cannot be null.";
/// `SR.ArgumentOutOfRange_StartIndexLargerThanLength`, from `String.Substring`.
pub const ARGUMENT_OUT_OF_RANGE_START_INDEX_LARGER_THAN_LENGTH: &str =
    "startIndex cannot be larger than length of string.";
pub const ARGUMENT_OUT_OF_RANGE_INDEX: &str =
    "Index was out of range. Must be non-negative and less than the size of the collection.";
pub const FORMAT_EMPTY_INPUT_STRING: &str = "Input string was either empty or contained only whitespace.";
pub const FORMAT_EXTRA_JUNK_AT_END: &str = "Additional non-parsable characters are at the end of the string.";
pub const FORMAT_GUID_BRACE: &str = "Expected {0xdddddddd, etc}.";
pub const FORMAT_GUID_BRACE_AFTER_LAST_NUMBER: &str =
    "Could not find a brace, or the length between the previous token and the brace was zero (i.e., '0x,'etc.).";
pub const FORMAT_GUID_COMMA: &str =
    "Could not find a comma, or the length between the previous token and the comma was zero (i.e., '0x,'etc.).";
pub const FORMAT_GUID_DASHES: &str = "Dashes are in the wrong position for GUID parsing.";
pub const FORMAT_GUID_END_BRACE: &str = "Could not find the ending brace.";
/// Used unformatted: `Guid` passes its argument with `ParseFailureKind.Format`, which ignores it.
pub const FORMAT_GUID_HEX_PREFIX: &str = "Expected hex 0x in '{0}'.";
pub const FORMAT_GUID_INV_LEN: &str =
    "Guid should contain 32 digits with 4 dashes (xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx).";
pub const FORMAT_GUID_INVALID_CHAR: &str = "Guid string should only contain hexadecimal characters.";
pub const FORMAT_GUID_UNRECOGNIZED: &str = "Unrecognized Guid format.";
pub const FORMAT_NO_PARSIBLE_DIGITS: &str = "Could not find any recognizable digits.";
/// `SR.MissingMethod_Name`, from `MissingMethodException.Message` when it has a class name.
pub const MISSING_METHOD_NAME: &str = "Method '{0}' not found.";
/// `SR.IO_PathNotFound_Path`: `DirectoryNotFoundException` for a missing folder, with the full path.
pub const IO_PATH_NOT_FOUND_PATH: &str = "Could not find a part of the path '{0}'.";
pub const OVERFLOW_BYTE: &str = "Value was either too large or too small for an unsigned byte.";
pub const OVERFLOW_INT16: &str = "Value was either too large or too small for an Int16.";
pub const OVERFLOW_INT32: &str = "Value was either too large or too small for an Int32.";
pub const OVERFLOW_INT64: &str = "Value was either too large or too small for an Int64.";
pub const OVERFLOW_NEGATIVE_UNSIGNED: &str =
    "The string was being parsed as an unsigned number and could not have a negative sign.";
pub const OVERFLOW_SBYTE: &str = "Value was either too large or too small for a signed byte.";
pub const OVERFLOW_UINT32: &str = "Value was either too large or too small for a UInt32.";
pub const OVERFLOW_UINT64: &str = "Value was either too large or too small for a UInt64.";

/// Every string above, for the install test.
pub const ALL: &[&str] = &[
    ARGUMENT_ADDING_DUPLICATE_WITH_KEY,
    ARG_ARGUMENT_OUT_OF_RANGE_EXCEPTION,
    ARG_CANNOT_HAVE_NEGATIVE_VALUE,
    ARG_HEX_STYLE_NOT_SUPPORTED,
    ARG_INDEX_OUT_OF_RANGE_EXCEPTION,
    ARG_INVALID_BASE,
    ARG_INVALID_CAST_EXCEPTION,
    ARG_INVALID_HEX_STYLE,
    ARG_NULL_REFERENCE_EXCEPTION,
    ARG_OVERFLOW_EXCEPTION,
    ARG_PARAM_NAME_NAME,
    ARGUMENT_INVALID_NUMBER_STYLES,
    ARGUMENT_NULL_GENERIC,
    ARGUMENT_OUT_OF_RANGE_START_INDEX_LARGER_THAN_LENGTH,
    ARGUMENT_OUT_OF_RANGE_INDEX,
    FORMAT_EMPTY_INPUT_STRING,
    FORMAT_EXTRA_JUNK_AT_END,
    FORMAT_GUID_BRACE,
    FORMAT_GUID_BRACE_AFTER_LAST_NUMBER,
    FORMAT_GUID_COMMA,
    FORMAT_GUID_DASHES,
    FORMAT_GUID_END_BRACE,
    FORMAT_GUID_HEX_PREFIX,
    FORMAT_GUID_INV_LEN,
    FORMAT_GUID_INVALID_CHAR,
    FORMAT_GUID_UNRECOGNIZED,
    FORMAT_NO_PARSIBLE_DIGITS,
    IO_PATH_NOT_FOUND_PATH,
    MISSING_METHOD_NAME,
    OVERFLOW_BYTE,
    OVERFLOW_INT16,
    OVERFLOW_INT32,
    OVERFLOW_INT64,
    OVERFLOW_NEGATIVE_UNSIGNED,
    OVERFLOW_SBYTE,
    OVERFLOW_UINT32,
    OVERFLOW_UINT64,
];
