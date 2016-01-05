//! Zend Memory Allocation Utilities
use std::mem;
use std::ops::{Placer, Place, InPlace, Deref, DerefMut};
use std::ptr;
use std::marker;
use std::fmt;
use ffi;

pub const ZEND_MM: ZendMMSingleton = ZendMMSingleton { _force_singleton: () };

#[derive(Copy, Clone)]
pub struct ZendMMSingleton {
    _force_singleton: (),
}

impl<T> Placer<T> for ZendMMSingleton {
    type Place = ZendBoxIntermediate<T>;

    fn make_place(self) -> ZendBoxIntermediate<T> {
        make_place()
    }
}

pub struct ZendBox<T>(pub *mut T);

impl<T> ZendBox<T> {
    #[inline]
    pub fn new(val: T) -> ZendBox<T> {
        let box_ = ZEND_MM <- val;
        box_
    }

    #[inline]
    pub fn into_raw(b: ZendBox<T>) -> *mut T {
        let ptr = b.0;
        mem::forget(b);
        ptr
    }
}

pub struct ZendBoxIntermediate<T> {
    ptr: *mut u8,
    size: usize,
    align: usize,
    marker: marker::PhantomData<*mut T>,
}

impl<T> Place<T> for ZendBoxIntermediate<T> {
    fn pointer(&mut self) -> *mut T {
        self.ptr as *mut T
    }
}

fn make_place<T>() -> ZendBoxIntermediate<T> {
    let size = mem::size_of::<T>();
    let align = mem::align_of::<T>();

    let p = if size == 0 {
        0x01 as *mut u8
    } else {
        let p = unsafe { zend_emalloc!(size) } as *mut u8;
        if p.is_null() {
            panic!("ZendBoxIntermediate make_place allocation failure");
        }
        p
    };

    ZendBoxIntermediate {
        ptr: p,
        size: size,
        align: align,
        marker: marker::PhantomData,
    }
}

impl<T> InPlace<T> for ZendBoxIntermediate<T> {
    type Owner = ZendBox<T>;
    unsafe fn finalize(self) -> ZendBox<T> {
        let p = self.ptr as *mut T;
        mem::forget(self);
        ZendBox(p)
    }
}

impl<T> Drop for ZendBoxIntermediate<T> {
    fn drop(&mut self) {
        panic!("when does this run?");
        if self.size > 0 {
            //unsafe { heap::deallocate(self.ptr, self.size, self.align) }
        }
    }
}

// ZendBox related stuff
impl<T: fmt::Debug > fmt::Debug for ZendBox<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl<T> Deref for ZendBox<T> {
    type Target = T;

    fn deref(&self) -> &T {
        unsafe { &*self.0 }
    }
}

impl<T> DerefMut for ZendBox<T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.0 }
    }
}

impl<T> Drop for ZendBox<T> {
    fn drop(&mut self) {
        unsafe {
            zend_free!(self.0 as *mut _);
        }
    }
}

// Refcounted Management
#[derive(Debug)]
#[repr(C)]
pub struct ZendRefcounted {
    pub refcount: u32,
    pub type_info: u32
}

#[derive(Debug)]
pub struct Refcounted<T>(pub ZendBox<T>);

impl Refcounted<ZendRefcounted> {
    /// Construct a `Refcounted` objec and destroy it using the drop method
    pub unsafe fn drop_ptr(p: *mut ZendRefcounted) {
        let obj = Refcounted(ZendBox(p));
        mem::drop(obj);
    }
}

impl<T> Refcounted<T> {
    #[inline]
    pub fn new(val: T) -> Refcounted<T> {
        let box_ = ZEND_MM <- val;
        Refcounted(box_)
    }

    #[inline]
    pub fn into_raw(b: Refcounted<T>) -> *mut T {
        let ptr = (b.0).0;
        mem::forget(b);
        ptr
    }
}

// Test that into raw doesn't call drop later
#[test]
fn test_into_raw() {
    let src_ptr = 0x666 as *mut u32;
    {
        let box_ = ZendBox(src_ptr);
        let ptr = ZendBox::into_raw(box_);
        assert_eq!(src_ptr, ptr);
    }
    {
        let box_ = ZendBox(src_ptr);
        let rc = Refcounted(box_);
        let ptr = Refcounted::into_raw(rc);
        assert_eq!(ptr, src_ptr);
    }
}

/// Ensures not to leak memory
impl<T> Drop for Refcounted<T> {
    fn drop(&mut self) {
        // Make sure we can access the refcounted structure
        let rc: &mut ZendRefcounted = unsafe { mem::transmute(&mut *(self.0)) };
        // If it's only referenced in this scope, we can kill it
        if rc.refcount <= 1 {
            // _zval_dtor_func is able to handle refcounted structures
            unsafe { zend_dtor!(mem::transmute(rc)); }
        } else {
            rc.refcount -= 1;
        }
        // do not free the underlying memory (it's freed from within zend when refcount <=1)
        (self.0).0 = ptr::null_mut();
    }
}

impl<T> Deref for Refcounted<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &*self.0
    }
}

impl<T> DerefMut for Refcounted<T> {
    fn deref_mut<'a>(&'a mut self) -> &mut T {
        &mut *self.0
    }
}
