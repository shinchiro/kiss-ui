//! A UI framework for Rust based on the KISS principle: "Keep It Simple, Stupid!"
//!
//! Built on top of the [IUP GUI library for C.][iup]
//!
//! ##Note: "valid KISS-UI context"
//! All KISS-UI static widget methods will panic if called before `kiss_ui::show_gui()` is invoked or
//! after it returns. 
//!
//! This is because the underlying IUP library has been either, respectively, not initialized yet 
//! or already deinitialized, and attempting to interact with it in either situation will likely cause
//! undefined behavior.
//!
//! ##Note: This is a (technically) leaky abstraction.
//! Because IUP only frees all its allocations when it is deinitialized, all widgets created by KISS-UI
//! will remain in-memory until `kiss_ui::show_gui()` returns. While unbounded memory growth can
//! happen with complex applications, this should not be an issue for most use-cases.
//!
//! However, some types *do* allocate large chunks of memory, or other valuable system resources, 
//! and should be manually freed when they are no longer being used. 
//! This is most evident with the `Image` struct, which can allocate large backing buffers for image data.
//!
//! All types that should be manually freed expose a `.destroy()` method which should be called
//! when they are no longer being used. This can safely be called multiple times on clones of the
//! widget types^([citation needed]).
//!
//! [iup]: http://webserver2.tecgraf.puc-rio.br/iup/

extern crate libc;
extern crate iup_sys;


macro_rules! assert_kiss_running (
    () => (
        assert!(
            ::kiss_running(), 
            "No KISS-UI widget methods may be called before `kiss_ui::show_gui()` is invoked or after it returns!"
        )
    )
);

#[macro_use]
pub mod base;

#[macro_use]
pub mod utils;

// Internal use modules
mod attrs;

// User-facing modules
#[macro_use]
pub mod callback;

pub mod button;
pub mod container;
pub mod dialog;
pub mod image;
pub mod progress;
pub mod text;
pub mod timer;

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::ptr;

use base::BaseWidget;

mod widget_prelude {
    pub use base::{BaseWidget, ImplDetails};
}

/// The entry point for KISS-UI. The closure argument should initialize and return the main window
/// dialog, at which point `.show()` will be called on it and the IUP event loop will begin
/// running.
///
/// ##Blocks
/// Until all KISS-UI dialogs are closed.
///
/// ##Warning
/// No static widget methods from this crate may be called before this function is
/// invoked or after it returns, with the exception of the closure passed to this function.
///
/// While this function is blocked and the IUP event loop is running, any reachable code is
/// considered a "valid KISS-UI context" and may create and interact with widgets and dialogs.
///
/// After it returns, IUP is deinitialized and all static widget methods will panic to avoid
/// undefined behavior.
pub fn show_gui<F>(init_fn: F) where F: FnOnce() -> dialog::Dialog {
    use ::utils::cstr::AsCStr;

    unsafe { 
        assert!(iup_sys::IupOpen(ptr::null(), ptr::null()) == 0);
        // Force IUP to always use UTF-8
        iup_sys::IupSetGlobal(::attrs::UTF8_MODE.as_cstr(), ::attrs::values::YES.as_cstr());
    }

    KISS_RUNNING.with(|state| state.set(true));

    init_fn().show();

    unsafe { 
        iup_sys::IupMainLoop();
        iup_sys::IupClose();
    }

    KISS_RUNNING.with(|state| state.set(false));

    // Evict the widget store and let it deallocate.
    WIDGET_STORE.with(|store| {
        *store.borrow_mut() = HashMap::new();
    });
}

fn kiss_running() -> bool {
    KISS_RUNNING.with(|state| state.get())
}

thread_local! { static KISS_RUNNING: Cell<bool> = Cell::new(false) }

thread_local! { static WIDGET_STORE: RefCell<HashMap<String, BaseWidget>> = RefCell::new(HashMap::new()) } 
