//! Arc-based copy-on-write container

use std::borrow::Borrow;
use std::hash::Hash;
use std::ops::Deref;
use std::sync::Arc;

/// A copy-on-write smart pointer backed by Arc
#[derive(Debug)]
pub enum ArcCow<'a, T: ?Sized> {
    Borrowed(&'a T),
    Owned(Arc<T>),
}

impl<T: ?Sized> Clone for ArcCow<'_, T> {
    fn clone(&self) -> Self {
        match self {
            ArcCow::Borrowed(b) => ArcCow::Borrowed(b),
            ArcCow::Owned(o) => ArcCow::Owned(Arc::clone(o)),
        }
    }
}

impl<T: ?Sized> Deref for ArcCow<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        match self {
            ArcCow::Borrowed(b) => b,
            ArcCow::Owned(o) => o.as_ref(),
        }
    }
}

impl<T: ?Sized> AsRef<T> for ArcCow<'_, T> {
    fn as_ref(&self) -> &T {
        self.deref()
    }
}

impl<T: ?Sized> Borrow<T> for ArcCow<'_, T> {
    fn borrow(&self) -> &T {
        self.deref()
    }
}

impl<T: ?Sized + PartialEq> PartialEq for ArcCow<'_, T> {
    fn eq(&self, other: &Self) -> bool {
        self.deref() == other.deref()
    }
}

impl<T: ?Sized + Eq> Eq for ArcCow<'_, T> {}

impl<T: ?Sized + PartialOrd> PartialOrd for ArcCow<'_, T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.deref().partial_cmp(other.deref())
    }
}

impl<T: ?Sized + Ord> Ord for ArcCow<'_, T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.deref().cmp(other.deref())
    }
}

impl<T: ?Sized + Hash> Hash for ArcCow<'_, T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.deref().hash(state)
    }
}

impl<'a, T: ?Sized> From<&'a T> for ArcCow<'a, T> {
    fn from(value: &'a T) -> Self {
        ArcCow::Borrowed(value)
    }
}

impl<T: ?Sized> From<Arc<T>> for ArcCow<'_, T> {
    fn from(value: Arc<T>) -> Self {
        ArcCow::Owned(value)
    }
}

impl From<String> for ArcCow<'_, str> {
    fn from(value: String) -> Self {
        ArcCow::Owned(Arc::from(value))
    }
}
