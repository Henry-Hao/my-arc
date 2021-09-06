use std::{marker::PhantomData, ops::{Deref, DerefMut}, ptr::NonNull, sync::atomic};

pub struct MyArc<T> {
    // ptr is variant of T
    ptr: NonNull<ArcInner<T>>,
    // as if we own the data T
    _marker: PhantomData<T>
}

pub struct ArcInner<T> {
    // Rc is used to record the last owner of this data, which could be used cross-thread.
    rc: atomic::AtomicUsize,
    data: T
}

// Bounds <T: Send + Sync> is requied as we don't want data races.
// e.g. MyArc<Rc<String>>, Rc is not thread-safe( T: !(Send+Sync)). If the bound is not present, Rc
// will be shared across threads where data race happens.
unsafe impl<T: Send + Sync> Send for MyArc<T> {}
unsafe impl<T: Send + Sync> Sync for MyArc<T> {}

impl<T> MyArc<T> {
    pub fn new(data: T) -> Self {
        let inner = ArcInner {
            rc: atomic::AtomicUsize::new(1),
            data
        };

        MyArc {
            ptr: NonNull::new(Box::into_raw(Box::new(inner))).unwrap(),
            _marker: PhantomData
        }
    }

    pub fn count(&self) -> usize {
        let inner = self.ptr.as_ptr();
        unsafe {
            (*inner).rc.load(atomic::Ordering::Acquire)
        }
    }
}

impl<T> Deref for MyArc<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        let inner = unsafe { self.ptr.as_ref() };
        &inner.data
    }
}

impl<T> DerefMut for MyArc<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        let inner = unsafe  { self.ptr.as_mut() };
        &mut inner.data
    }
}

impl<T> Clone for MyArc<T> {
    fn clone(&self) -> Self {
        let inner = unsafe { self.ptr.as_ref() };
        // use Ordering::Relaxed because we don't need any synchronization.
        let old_rc = inner.rc.fetch_add(1, atomic::Ordering::Relaxed);
        // In the case that someone cloned MyArc then use std::mem::forget to forget it without
        // running the destructor(decrease rc), the memory will be overflowed. So a threshold is
        // necessary.
        if old_rc >= isize::MAX as usize {
            std::process::abort();
        }

        Self {
            ptr: self.ptr,
            _marker: PhantomData
        }
    }
}

/// 1. decrease rc if it is greater than 1
/// 2. if rc equals to 1(only one reference remaining)
///     - 1. set a barrier to prevernt reorder of use and deletion of the data
///     - 2. drop inner data
impl<T> Drop for MyArc<T> {
    fn drop(&mut self) {
        let inner = unsafe { self.ptr.as_ref() };
        if (*inner).rc.fetch_sub(1, atomic::Ordering::Release) != 1 {
            return;
        }
        atomic::fence(atomic::Ordering::Acquire);

        unsafe {
            Box::from_raw(self.ptr.as_ptr());
        }
    }
}

impl<T> Drop for ArcInner<T> {
    fn drop(&mut self) {

    }
}

#[cfg(test)]
mod tests {
    use crate::MyArc;
    #[test]
    fn test_new() {
        let a = MyArc::new(1);
        assert!(std::mem::size_of_val(&a) != 0)
    }

    #[test]
    fn test_deref() {
        let a = MyArc::new(1);
        assert_eq!(*a, 1);
    }

    #[test]
    fn test_deref_mut() {
        let mut a = MyArc::new(1);
        *a = 3;
        assert_eq!(*a, 3);
    }

    #[test]
    fn test_clone() {
        let a = MyArc::new(0);
        assert_eq!(a.count(), 1);
        let b = a.clone();
        assert_eq!(a.count(), 2);
    }

    #[test]
    fn test_drop() {
        let a = MyArc::new(2);
        assert_eq!(a.count(), 1);
        let b = a.clone();
        assert_eq!(a.count(), 2);
        drop(a);
        assert_eq!(b.count(), 1);
    }

}
