use std::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
    ptr::NonNull,
};

pub struct Foreign<'wren, T> {
    data: NonNull<T>,
    // We don't actually own this data, the VM does
    // it's also not guarenteed to be alive past the
    // foreign method that it is called on
    phantom: PhantomData<&'wren mut T>,
}

impl<'wren, T> Foreign<'wren, T> {
    pub(super) unsafe fn new(data: NonNull<T>) -> Self {
        Self {
            data,
            phantom: PhantomData::default(),
        }
    }
}

impl<'wren, T> AsRef<T> for Foreign<'wren, T> {
    fn as_ref(&self) -> &T {
        unsafe { &*self.data.as_ptr() }
    }
}

impl<'wren, T> AsMut<T> for Foreign<'wren, T> {
    fn as_mut(&mut self) -> &mut T {
        unsafe { &mut *self.data.as_ptr() }
    }
}

impl<'wren, T> Deref for Foreign<'wren, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl<'wren, T> DerefMut for Foreign<'wren, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut()
    }
}
