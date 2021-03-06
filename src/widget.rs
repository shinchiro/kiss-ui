//! Operations common to all widget types.

use utils::cstr::AsCStr;

use base::{BaseWidget, Downcast};
use dialog::Dialog;

use iup_sys;

use std::ffi::{CStr, CString};
use std::ptr;

/// Trait implemented for all widget types.
///
/// Some methods may not apply to some widgets.
pub trait Widget: IUPWidget {
    /// Show this widget if it was previously hidden.
    ///
    /// Does nothing if the widget is already shown, or if the operation does not apply.
    fn show(self) -> Self {
        unsafe { iup_sys::IupShow(self.ptr()); }
        self
    }

    /// Hide this widget if it was previously visible.
    ///
    /// Does nothing if the widget is already hidden, or if the operation does not apply.
    fn hide(self) -> Self {
        unsafe { iup_sys::IupHide(self.ptr()); }
        self
    }

    /// Set the widget's visibility state.
    ///
    /// `.set_visible(true)` is equivalent to calling `.show()`, and `.set_visible(false)`
    /// is equivalent to calling `.hide()`.
    ///
    /// Does nothing if the widget is in the same visibility state as the one being set,
    /// or if the operation does not apply.
    fn set_visible(self, visible: bool) -> Self {
        self.set_bool_attribute(::attrs::VISIBLE, visible);
        self
    }

    /// Set the widget's enabled state.
    ///
    /// When a widget is disabled, it does not react to user interaction or invoke any callbacks.
    ///
    /// Does nothing if the widget does not support being disabled.
    fn set_enabled(self, enabled: bool) -> Self {
        self.set_bool_attribute(::attrs::ACTIVE, enabled);
        self
    }

    /// Set the position of this widget relative to the top-left corner of its parent.
    ///
    /// Does nothing if the widget is not renderable or not attached to a parent.
     fn set_position(self, x: i32, y: i32) -> Self {
        self.set_str_attribute(::attrs::POSITION, format!("{x},{y}", x=x, y=y));
        self
    }

    /// Get the position of this widget relative to the top-left corner of its parent.
    ///
    /// Returns (0, 0) if the widget is not renderable, not attached to a parent, or if that is the
    /// widget's actual relative position.
    fn get_position(self) -> (i32, i32) {
        self.get_int2_attribute(::attrs::POSITION)
    }

    /// Set the name of the widget so it can be found within its parent.
    ///
    /// Does nothing if the widget does not support having a name.
    fn set_name(self, name: &str) -> Self {
        self.set_str_attribute(::attrs::NAME, name);
        self
    }

    /// Get the name of this widget, if the widget supports having a name and one is set.
    fn get_name(&self) -> Option<&str> {
        self.get_str_attribute(::attrs::NAME) 
    }  

    /// Get the next child in the parent after this widget, based on the order in which they were 
    /// added.
    ///
    /// Returns `None` if this widget is an only child or is not attached to a parent.
    fn get_sibling(self) -> Option<BaseWidget> {
        unsafe {
            let ptr = iup_sys::IupGetBrother(self.ptr());
            BaseWidget::from_ptr_opt(ptr)
        }
    }

    /// Get the parent of this widget.
    ///
    /// Returns `None` if this widget has no parent.
    fn get_parent(self) -> Option<BaseWidget> {
        unsafe {
            let ptr = iup_sys::IupGetParent(self.ptr());
            BaseWidget::from_ptr_opt(ptr)
        }
    }

    /// Get the containing dialog of this widget.
    ///
    /// Returns `None` if this widget is not attached to a dialog.
    fn get_dialog(self) -> Option<Dialog> {
        unsafe {
            let ptr = iup_sys::IupGetDialog(self.ptr());
            // Note to self: not using UFCS because `downcast()` is an unsafe function.
            BaseWidget::from_ptr_opt(ptr).map(|base| Dialog::downcast(base))
        }
    }

    /// Get the rendered size of this widget, in pixels.
    ///
    /// Returns `(0, 0)` if this widget has no rendered size.
    fn get_size_pixels(self) -> (u32, u32) {
        let (width, height) = self.get_int2_attribute(::attrs::RASTERSIZE);
        (width as u32, height as u32)
    }

    /// Store this widget under `name`, returning the previous widget stored, if any.
    ///
    /// It may later be retrieved from any valid KISS-UI context 
    /// by calling `BaseWidget::load(name)`.
    fn store<N: Into<String>>(self, name: N) -> Option<BaseWidget> {
        ::WIDGET_STORE.with(|store| {
            store.borrow_mut().insert(name.into(), self.to_base())
        })
    }

    fn to_base(self) -> BaseWidget {
        unsafe { BaseWidget::from_ptr(self.ptr()) }
    }
}


#[doc(hidden)]
impl<T: IUPWidget> Widget for T {}

pub trait Destroy: Widget {
    fn destroy(self) {
        unsafe { iup_sys::IupDestroy(self.ptr()); }
    }
}

#[doc(hidden)]
pub trait IUPWidget: Copy {
    unsafe fn from_ptr(ptr: *mut iup_sys::Ihandle) -> Self;

    unsafe fn from_ptr_opt(ptr: *mut iup_sys::Ihandle) -> Option<Self> {
        if !ptr.is_null() {
            Some(Self::from_ptr(ptr))
        } else {
            None
        }
    }

    fn ptr(self) -> *mut iup_sys::Ihandle;

    fn target_classname() -> &'static str;

    fn classname(&self) -> &CStr {
        unsafe { CStr::from_ptr(iup_sys::IupGetClassName(self.ptr())) } 
    }

    fn set_str_attribute<V>(self, name: &'static str, val: V) where V: Into<String> {
        let c_val = CString::new(val.into()).unwrap();
        unsafe { iup_sys::IupSetStrAttribute(self.ptr(), name.as_cstr(), c_val.as_ptr()); }
    }

    fn set_opt_str_attribute<V>(self, name: &'static str, val: Option<V>) where V: Into<String> {
        let c_val = val.map(V::into).map(CString::new).map(Result::unwrap);
        unsafe { 
            iup_sys::IupSetStrAttribute(
                self.ptr(),
                name.as_cstr(),
                // This looks backwards, but check the docs. It's right.
                c_val.as_ref().map_or_else(ptr::null, |c_val| c_val.as_ptr())
            )
        }
    }

    fn set_const_str_attribute(self, name: &'static str, val: &'static str) {
        unsafe { iup_sys::IupSetAttribute(self.ptr(), name.as_cstr(), val.as_cstr()); }
    }

    fn get_str_attribute(&self, name: &'static str) -> Option<&str> {
        let ptr = unsafe { iup_sys::IupGetAttribute(self.ptr(), name.as_cstr()) };

        if !ptr.is_null() {
            unsafe {
                // Safe since we're controlling the lifetime
                let c_str = CStr::from_ptr(ptr);
                // We're forcing IUP to use UTF-8 
                Some(::std::str::from_utf8_unchecked(c_str.to_bytes()))
            }
        } else {
            None
        }
    }

    fn set_int_attribute(self, name: &'static str, val: i32) {
        unsafe { iup_sys::IupSetInt(self.ptr(), name.as_cstr(), val); }
    }

    fn get_int_attribute(self, name: &'static str) -> i32 {
        unsafe { iup_sys::IupGetInt(self.ptr(), name.as_cstr()) }
    }

    fn get_int2_attribute(self, name: &'static str) -> (i32, i32) {
        let mut left = 0;
        let mut right = 0;

        unsafe { 
            assert!(iup_sys::IupGetIntInt(self.ptr(), name.as_cstr(), &mut left, &mut right) != 0); 
        }

        (left, right)
    }

    fn set_float_attribute(self, name: &'static str, val: f32) {
        unsafe { iup_sys::IupSetFloat(self.ptr(), name.as_cstr(), val); } 
    }

    fn get_float_attribute(self, name: &'static str) -> f32 {
        unsafe { iup_sys::IupGetFloat(self.ptr(), name.as_cstr()) }
    }

    fn set_bool_attribute(self, name: &'static str, val: bool) {
        let val = ::attrs::values::bool_yes_no(val);
        self.set_const_str_attribute(name, val);        
    }

    fn set_attr_handle<W: Widget>(self, name: &'static str, handle: W) {
        unsafe { iup_sys::IupSetAttributeHandle(self.ptr(), name.as_cstr(), handle.ptr()); }
    }

    fn get_attr_handle(self, name: &'static str) -> Option<BaseWidget> {
        unsafe { 
            let existing = iup_sys::IupGetAttributeHandle(self.ptr(), name.as_cstr());
            BaseWidget::from_ptr_opt(existing)
        }
    }

    fn set_callback(self, name: &'static str, callback: ::iup_sys::Icallback) {
        unsafe { iup_sys::IupSetCallback(self.ptr(), name.as_cstr(), callback); } 
    }    
}

impl<'a, T: IUPWidget> IUPWidget for &'a T {
    unsafe fn from_ptr(_ptr: *mut iup_sys::Ihandle) -> Self {
        panic!("Cannot construct an &mut Self from a pointer");
    }

    fn ptr(self) -> *mut iup_sys::Ihandle {
        (*self).ptr()
    }

    fn target_classname() -> &'static str {
        T::target_classname()
    }
}

#[macro_export]
macro_rules! impl_widget {
    ($ty:ident, $classname:expr) => {
        impl ::widget::IUPWidget for $ty {
            unsafe fn from_ptr(ptr: ::widget_prelude::IUPPtr) -> Self {
                assert!(
                    !ptr.is_null(), 
                    concat!(
                        concat!("Failed to construct ", stringify!($ty)),
                        "; pointer returned from IUP was null!"
                    )                    
                );

                $ty(ptr)
            }

            fn ptr(self) -> ::widget_prelude::IUPPtr {
                self.0
            }

            fn target_classname() -> &'static str {
                $classname
            }            
        }
    
        impl Copy for $ty {}
        
        impl Clone for $ty {
            fn clone(&self) -> Self {
                *self
            }
        }
    }
}
