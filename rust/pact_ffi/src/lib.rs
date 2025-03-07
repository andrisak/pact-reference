//! A crate exposing the `pact` APIs to other languages
//! via a C Foreign Function Interface.

#![warn(missing_docs)]
#![warn(missing_debug_implementations)]
#![warn(missing_copy_implementations)]

use std::ffi::CStr;
use std::str::FromStr;

use libc::c_char;
use pact_models::interaction::Interaction;
use pact_models::pact::Pact;
use pact_models::v4::pact::V4Pact;
use tracing::{debug, error, info, trace, warn};
use tracing_core::Level;
use tracing_log::AsLog;
use tracing_subscriber::FmtSubscriber;

use models::message::Message;
use pact_matching as pm;
pub use pact_matching::Mismatch;

use crate::util::*;

pub mod error;
pub mod log;
pub mod models;
pub(crate) mod util;
pub mod mock_server;
pub mod verifier;
pub mod plugins;
pub mod matching;

const VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), "\0");

/// Returns the current library version
#[no_mangle]
pub extern "C" fn pactffi_version() -> *const c_char {
    VERSION.as_ptr() as *const c_char
}

/// Initialise the mock server library, can provide an environment variable name to use to
/// set the log levels. This function should only be called once, as it tries to install a global
/// tracing subscriber.
///
/// # Safety
///
/// log_env_var must be a valid NULL terminated UTF-8 string.
#[no_mangle]
pub unsafe extern fn pactffi_init(log_env_var: *const c_char) {
    let log_env_var = if !log_env_var.is_null() {
        let c_str = CStr::from_ptr(log_env_var);
        match c_str.to_str() {
            Ok(str) => str,
            Err(err) => {
                warn!("Failed to parse the environment variable name as a UTF-8 string: {}", err);
                "LOG_LEVEL"
            }
        }
    } else {
        "LOG_LEVEL"
    };

    let subscriber = FmtSubscriber::builder()
      .with_env_filter(log_env_var)
      .with_thread_names(true)
      .with_ansi(false) // Pact .Net can't deal with ANSI escape codes
      .finish();
    if let Err(err) = tracing::subscriber::set_global_default(subscriber) {
      eprintln!("Failed to initialise global tracing subscriber - {err}");
    };
}

/// Initialises logging, and sets the log level explicitly. This function should only be called
/// once, as it tries to install a global tracing subscriber.
///
/// # Safety
///
/// Exported functions are inherently unsafe.
#[no_mangle]
pub unsafe extern "C" fn pactffi_init_with_log_level(level: *const c_char) {
  let log_level = log_level_from_c_char(level);
  let subscriber = FmtSubscriber::builder()
    .with_max_level(log_level)
    .with_thread_names(true)
    .with_ansi(false) // Pact .Net can't deal with ANSI escape codes
    .finish();
  if let Err(err) = tracing::subscriber::set_global_default(subscriber) {
    eprintln!("Failed to initialise global tracing subscriber - {err}");
  };
}

/// Enable ANSI coloured output on Windows. On non-Windows platforms, this function is a no-op.
///
/// # Safety
///
/// This function is safe.
#[no_mangle]
#[cfg(windows)]
pub extern "C" fn pactffi_enable_ansi_support() {
  if let Err(err) = ansi_term::enable_ansi_support() {
    warn!("Could not enable ANSI console support - {err}");
  }
}

/// Enable ANSI coloured output on Windows. On non-Windows platforms, this function is a no-op.
///
/// # Safety
///
/// This function is safe.
#[no_mangle]
#[cfg(not(windows))]
pub extern "C" fn pactffi_enable_ansi_support() { }

/// Log using the shared core logging facility.
///
/// This is useful for callers to have a single set of logs.
///
/// * `source` - String. The source of the log, such as the class or caller framework to
///                      disambiguate log lines from the rust logging (e.g. pact_go)
/// * `log_level` - String. One of TRACE, DEBUG, INFO, WARN, ERROR
/// * `message` - Message to log
///
/// # Safety
/// This function will fail if any of the pointers passed to it are invalid.
#[no_mangle]
pub unsafe extern "C" fn pactffi_log_message(source: *const c_char, log_level: *const c_char, message: *const c_char) {
  let target = convert_cstr("target", source).unwrap_or("client");

  if !message.is_null() {
    if let Some(message) = convert_cstr("message", message) {
      match log_level_from_c_char(log_level).as_log() {
        ::log::Level::Error => error!(source = target, "{}", message),
        ::log::Level::Warn => warn!(source = target, "{}", message),
        ::log::Level::Info => info!(source = target, "{}", message),
        ::log::Level::Debug => debug!(source = target, "{}", message),
        ::log::Level::Trace => trace!(source = target, "{}", message)
      }
    }
  }
}

unsafe fn log_level_from_c_char(log_level: *const c_char) -> Level {
  if !log_level.is_null() {
    let level = convert_cstr("log_level", log_level).unwrap_or("INFO");
    Level::from_str(level).unwrap_or(tracing::Level::INFO)
  } else {
    Level::INFO
  }
}

fn convert_cstr(name: &str, value: *const c_char) -> Option<&str> {
    unsafe {
        if value.is_null() {
            warn!("{} is NULL!", name);
            None
        } else {
            let c_str = CStr::from_ptr(value);
            match c_str.to_str() {
                Ok(str) => Some(str),
                Err(err) => {
                    warn!("Failed to parse {} name as a UTF-8 string: {}", name, err);
                    None
                }
            }
        }
    }
}

ffi_fn! {
    /// Match a pair of messages, producing a collection of mismatches,
    /// which is empty if the two messages matched.
    fn pactffi_match_message(msg_1: *const Message, msg_2: *const Message) -> *const Mismatches {
        let msg_1: Box<dyn Interaction + Send + Sync> = unsafe { Box::from_raw(msg_1 as *mut Message) };
        let msg_2: Box<dyn Interaction + Send + Sync> = unsafe { Box::from_raw(msg_2 as *mut Message) };

        let runtime = tokio::runtime::Runtime::new().unwrap();
        let mismatches = runtime.block_on(async move {
            // TODO: match_message also requires the Pact that the messages belong to
            Mismatches(pm::match_message(&msg_1, &msg_2, &V4Pact::default().boxed()).await)
        });

        ptr::raw_to(mismatches) as *const Mismatches
    } {
        ptr::null_to::<Mismatches>() as *const Mismatches
    }
}

ffi_fn! {
    /// Get an iterator over mismatches.
    fn pactffi_mismatches_get_iter(mismatches: *const Mismatches) -> *mut MismatchesIterator {
        let mismatches = as_ref!(mismatches);
        let iter = MismatchesIterator { current: 0, mismatches };
        ptr::raw_to(iter)
    } {
        ptr::null_mut_to::<MismatchesIterator>()
    }
}

ffi_fn! {
    /// Delete mismatches
    fn pactffi_mismatches_delete(mismatches: *const Mismatches) {
        ptr::drop_raw(mismatches as *mut Mismatches);
    }
}

ffi_fn! {
    /// Get the next mismatch from a mismatches iterator.
    ///
    /// Returns a null pointer if no mismatches remain.
    fn pactffi_mismatches_iter_next(iter: *mut MismatchesIterator) -> *const Mismatch {
        let iter = as_mut!(iter);
        let mismatches = as_ref!(iter.mismatches);
        let index = iter.next();
        let mismatch = mismatches
            .0
            .get(index)
            .ok_or(anyhow::anyhow!("iter past the end of mismatches"))?;
       mismatch as *const Mismatch
    } {
        ptr::null_to::<Mismatch>()
    }
}

ffi_fn! {
    /// Delete a mismatches iterator when you're done with it.
    fn pactffi_mismatches_iter_delete(iter: *mut MismatchesIterator) {
        ptr::drop_raw(iter);
    }
}

ffi_fn! {
    /// Get a JSON representation of the mismatch.
    fn pactffi_mismatch_to_json(mismatch: *const Mismatch) -> *const c_char {
        let mismatch = as_ref!(mismatch);
        let json = mismatch.to_json().to_string();
        string::to_c(&json)? as *const c_char
    } {
        ptr::null_to::<c_char>()
    }
}

ffi_fn! {
    /// Get the type of a mismatch.
    fn pactffi_mismatch_type(mismatch: *const Mismatch) -> *const c_char {
        let mismatch = as_ref!(mismatch);
        let t = mismatch.mismatch_type();
        string::to_c(t)? as *const c_char
    } {
        ptr::null_to::<c_char>()
    }
}

ffi_fn! {
    /// Get a summary of a mismatch.
    fn pactffi_mismatch_summary(mismatch: *const Mismatch) -> *const c_char {
        let mismatch = as_ref!(mismatch);
        let summary = mismatch.summary();
        string::to_c(&summary)? as *const c_char
    } {
        ptr::null_to::<c_char>()
    }
}

ffi_fn! {
    /// Get a description of a mismatch.
    fn pactffi_mismatch_description(mismatch: *const Mismatch) -> *const c_char {
        let mismatch = as_ref!(mismatch);
        let description = mismatch.description();
        string::to_c(&description)? as *const c_char
    } {
        ptr::null_to::<c_char>()
    }
}

ffi_fn! {
    /// Get an ANSI-compatible description of a mismatch.
    fn pactffi_mismatch_ansi_description(mismatch: *const Mismatch) -> *const c_char {
        let mismatch = as_ref!(mismatch);
        let ansi_description = mismatch.ansi_description();
        string::to_c(&ansi_description)? as *const c_char
    } {
        ptr::null_to::<c_char>()
    }
}

/// A collection of mismatches from a matching comparison.
#[allow(missing_copy_implementations)]
#[allow(missing_debug_implementations)]
pub struct Mismatches(Vec<Mismatch>);

/// An iterator over mismatches.
#[allow(missing_copy_implementations)]
#[allow(missing_debug_implementations)]
pub struct MismatchesIterator {
    current: usize,
    mismatches: *const Mismatches,
}

impl MismatchesIterator {
    fn next(&mut self) -> usize {
        let idx = self.current;
        self.current += 1;
        idx
    }
}
