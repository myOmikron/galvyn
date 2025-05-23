use std::any::Any;
use std::panic;
use std::panic::Location;

use tracing::error;

/// Sets the global panic hook to output tracing events instead of writing to stdout
///
/// This function will be called implicitly by [`Galvyn::new`](crate::galvyn::Galvyn::new)
pub fn set_panic_hook() {
    panic::set_hook(Box::new(panic_hook))
}

/// The panic hook set by [`set_panic_hook`]
pub fn panic_hook(info: &panic::PanicHookInfo) {
    let msg = payload_as_str(info.payload());
    let location = info.location();

    error!(
        panic.file = location.map(Location::file),
        panic.line = location.map(Location::line),
        panic.column = location.map(Location::column),
        panic.msg = msg,
        "Panic"
    );
}

/// Copied from the std's default hook (v1.81.0)
fn payload_as_str(payload: &dyn Any) -> &str {
    if let Some(&s) = payload.downcast_ref::<&'static str>() {
        s
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.as_str()
    } else {
        "Box<dyn Any>"
    }
}
