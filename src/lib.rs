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
}
