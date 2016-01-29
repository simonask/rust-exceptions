extern crate libc;

use std::mem;
use std::any::Any;
use std::ffi::CStr;

#[cfg(test)]
use std::borrow::Borrow;

pub trait Exception : Any {
    fn what(&self) -> &str;
}

pub trait Rethrow {
    fn rethrow(self) -> !;
}

pub trait UnwrapOrRethrow<T> {
    fn unwrap_or_rethrow(self) -> T;
}


#[repr(C)]
struct FakeTraitObject {
    p0: *mut libc::c_void, // for C++ exceptions, this is an owned pointer to the exception.
    p1: *mut libc::c_void,
}

#[link(name = "cpp_exceptions_wrapper")]
extern {
    fn cpp_try(block: extern fn(*mut libc::c_void),
               state: *mut libc::c_void,
               caught_rust: *mut bool) -> FakeTraitObject;

    fn cpp_throw_rust(exception: FakeTraitObject) -> !;
    // fn cpp_rethrow() -> !;
    fn cpp_exception_what(exception: *mut libc::c_void) -> *const libc::c_char;
    fn cpp_exception_destroy(exception: *mut libc::c_void);

    #[cfg(test)]
    fn cpp_throw_test_exception(message: *const libc::c_char) -> !;
}

struct NativeCppExceptionWrapper {
    exception: *mut libc::c_void
}

impl Drop for NativeCppExceptionWrapper {
    fn drop(&mut self) {
        unsafe {
            cpp_exception_destroy(self.exception);
        }
    }
}

impl Exception for NativeCppExceptionWrapper {
    fn what(&self) -> &str {
        unsafe {
            let c_str = cpp_exception_what(self.exception);
            CStr::from_ptr(c_str).to_str().unwrap()
        }
    }
}
struct ThrowState<T, F: FnOnce() -> T> {
    try_block: Option<F>,
    returned_value: Option<T>
}

extern fn try_internal<T, F: FnOnce() -> T>(state: *mut ThrowState<T, F>) {
    let borrowed_state: &mut ThrowState<T, F> = unsafe {
        mem::transmute(state)
    };

    let value = (borrowed_state.try_block.take().unwrap())();
    borrowed_state.returned_value = Some(value);
}

pub fn try<T, F: FnOnce() -> T>(func: F) -> Result<T, Box<Exception>> {
    let mut state = ThrowState {
        try_block: Some(func),
        returned_value: None
    };
    let mut caught_rust = false;
    let exception = unsafe {
        let callback = try_internal::<T, F>;
        let borrowed_state = &mut state;
        cpp_try(mem::transmute(callback),
                mem::transmute(borrowed_state),
                mem::transmute(&mut caught_rust))
    };

    state.returned_value.ok_or_else(|| {
        if caught_rust {
            unsafe {
                let ex: *mut Exception = mem::transmute(exception);
                Box::<Exception>::from_raw(ex)
            }
        } else {
            let ex = NativeCppExceptionWrapper { exception: exception.p0 };
            let bex: Box<Exception> = Box::new(ex);
            bex
        }
    })
}

fn throw_boxed_exception(boxed: Box<Exception>) -> ! {
    let ex: FakeTraitObject = unsafe { mem::transmute(Box::into_raw(boxed)) };
    unsafe { cpp_throw_rust(ex) }
}

pub fn throw<T: Exception>(exception: T) -> ! {
    let boxed: Box<Exception> = Box::new(exception);
    throw_boxed_exception(boxed)
}

#[cfg(test)]
struct TestException {
    message: String
}

#[cfg(test)]
impl Exception for TestException {
    fn what(&self) -> &str {
        self.message.as_ref()
    }
}

#[test]
fn test_cpp_unwind() {
    let result = try(|| {
        throw(TestException{message: "Hello, World!".into()});
    });
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().what(), "Hello, World!");
}

#[cfg(test)]
struct Droppable<'a> {
    dropped: &'a mut bool
}

#[cfg(test)]
impl<'a> Drop for Droppable<'a> {
    fn drop(&mut self) {
        *self.dropped = true
    }
}

#[test]
fn test_exception_unwind_calls_drop() {
    let mut dropped = false;
    let result = try(|| {
        let droppable = Droppable{dropped: &mut dropped};
        assert!(!*droppable.dropped);
        throw(TestException{message: "Dropped!".into()});
    });
    assert!(result.is_err());
    assert!(dropped);
}

#[test]
fn test_catch_cpp_exception() {
    let result = try(|| {
        unsafe {
            let message = std::ffi::CString::new("Hello from C++!").unwrap();
            let msg_cstr: &CStr = message.borrow();
            cpp_throw_test_exception(msg_cstr.as_ptr());
        }
    });
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().what(), "Hello from C++!");
}

