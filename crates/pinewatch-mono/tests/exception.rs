//! Exception messages as Unity's Mono builds them (`docs/VERIFICATION.md`, V2).

use pinewatch_mono::{ExceptionKind, MonoException, NEW_LINE, sr};

#[test]
fn new_line_is_crlf_on_windows() {
    if cfg!(windows) {
        assert_eq!(NEW_LINE, "\r\n");
    }
}

#[test]
fn parameter_name_follows_environment_new_line() {
    assert_eq!(
        MonoException::argument_null("key").message,
        format!("Value cannot be null.{NEW_LINE}Parameter name: key")
    );
    assert_eq!(
        MonoException::argument_out_of_range("startIndex").message,
        format!("Specified argument was out of the range of valid values.{NEW_LINE}Parameter name: startIndex")
    );
    assert_eq!(
        MonoException::list_index_out_of_range().message,
        format!(
            "Index was out of range. Must be non-negative and less than the size of the collection.{NEW_LINE}Parameter name: index"
        )
    );
    assert_eq!(
        MonoException::argument(sr::ARG_INVALID_BASE, Some("radix")).message,
        format!("Invalid Base.{NEW_LINE}Parameter name: radix")
    );
}

#[test]
fn null_or_empty_parameter_name_adds_nothing() {
    assert_eq!(MonoException::argument("Message.", None).message, "Message.");
    assert_eq!(MonoException::argument("Message.", Some("")).message, "Message.");
    assert_eq!(MonoException::argument_out_of_range_default().message, sr::ARG_ARGUMENT_OUT_OF_RANGE_EXCEPTION);
}

#[test]
fn runtime_exceptions_use_the_default_constructor_messages() {
    assert_eq!(MonoException::null_reference().message, "Object reference not set to an instance of an object.");
    assert_eq!(MonoException::invalid_cast().message, "Specified cast is not valid.");
    assert_eq!(MonoException::overflow().message, "Arithmetic operation resulted in an overflow.");
    assert_eq!(MonoException::index_out_of_range().message, "Index was outside the bounds of the array.");
}

#[test]
fn display_names_the_type_and_the_inner_exception() {
    let e = MonoException::format(sr::FORMAT_GUID_UNRECOGNIZED)
        .with_inner(MonoException::argument(sr::ARG_CANNOT_HAVE_NEGATIVE_VALUE, None));
    assert_eq!(
        e.to_string(),
        "System.FormatException: Unrecognized Guid format. ---> System.ArgumentException: \
         String cannot contain a minus sign if the base is not 10."
    );
    assert_eq!(e.inner.as_deref().map(|inner| inner.kind), Some(ExceptionKind::Argument));
}
