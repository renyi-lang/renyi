//! A vector with a fixed layout (decision AR4): the pointer to its items,
//! their count and the capacity, in that order, so that the generated code
//! reads and writes the VM's stack and its frames in place by their
//! addresses. The common operations work on the three words here; the
//! uncommon ones make a `Vec` of the parts, use it and take it apart again.

use std::hash::{Hash, Hasher};
use std::mem::ManuallyDrop;
use std::ops::{Deref, DerefMut};

#[repr(C)]
pub struct Pinned<T> {
    ptr: *mut T,
    len: usize,
    cap: usize,
}

impl<T> Pinned<T> {
    pub fn new() -> Pinned<T> {
        Pinned::from(Vec::new())
    }

    pub fn with_capacity(capacity: usize) -> Pinned<T> {
        Pinned::from(Vec::with_capacity(capacity))
    }

    /// The parts as a `Vec`; the vector here is empty until `put`.
    fn take(&mut self) -> Vec<T> {
        // SAFETY: the parts came from a `Vec` through `put`, and the
        // vector is emptied so that nothing else frees them.
        let vec = unsafe { Vec::from_raw_parts(self.ptr, self.len, self.cap) };
        let empty = ManuallyDrop::new(Vec::<T>::new());
        self.ptr = empty.as_ptr() as *mut T;
        self.len = 0;
        self.cap = 0;
        vec
    }

    fn put(&mut self, vec: Vec<T>) {
        debug_assert_eq!(self.cap, 0, "the parts were taken first");
        let mut vec = ManuallyDrop::new(vec);
        self.ptr = vec.as_mut_ptr();
        self.len = vec.len();
        self.cap = vec.capacity();
    }

    /// Something done with the parts as a `Vec`.
    fn with_vec<R>(&mut self, action: impl FnOnce(&mut Vec<T>) -> R) -> R {
        let mut vec = self.take();
        let result = action(&mut vec);
        self.put(vec);
        result
    }

    /// Room for more: the capacity doubled (at least sixteen), for a
    /// push the generated code found no room for.
    pub fn grow(&mut self) {
        self.with_vec(|vec| vec.reserve(vec.len().max(16)));
    }

    pub fn as_slice(&self) -> &[T] {
        self
    }

    /// Room for `needed` items in all.
    pub fn room(&mut self, needed: usize) {
        if needed > self.cap {
            self.with_vec(|vec| vec.reserve(needed - vec.len()));
        }
    }

    #[inline]
    pub fn push(&mut self, value: T) {
        if self.len == self.cap {
            self.with_vec(|vec| vec.reserve(1));
        }
        // SAFETY: `len < cap`, so the slot is allocated and not in use.
        unsafe { self.ptr.add(self.len).write(value) };
        self.len += 1;
    }

    #[inline]
    pub fn pop(&mut self) -> Option<T> {
        if self.len == 0 {
            return None;
        }
        self.len -= 1;
        // SAFETY: the item at `len` was in use and is counted out first.
        Some(unsafe { self.ptr.add(self.len).read() })
    }

    /// Drop the items from `len` on; nothing when there are fewer.
    pub fn truncate(&mut self, len: usize) {
        while self.len > len {
            self.len -= 1;
            // SAFETY: the item was in use and is counted out first.
            unsafe { std::ptr::drop_in_place(self.ptr.add(self.len)) };
        }
    }

    pub fn insert(&mut self, index: usize, value: T) {
        assert!(
            index <= self.len,
            "the insertion point is inside the vector"
        );
        self.room(self.len + 1);
        // SAFETY: the items from `index` on move up one place, into the
        // room just made, and the slot they leave takes the value.
        unsafe {
            std::ptr::copy(
                self.ptr.add(index),
                self.ptr.add(index + 1),
                self.len - index,
            );
            self.ptr.add(index).write(value);
        }
        self.len += 1;
    }

    pub fn remove(&mut self, index: usize) -> T {
        self.with_vec(|vec| vec.remove(index))
    }

    pub fn extend(&mut self, items: impl IntoIterator<Item = T>) {
        for item in items {
            self.push(item);
        }
    }

    /// The items from `at` on, moved out in order.
    pub fn split_off(&mut self, at: usize) -> Vec<T> {
        assert!(at <= self.len, "the split is inside the vector");
        let count = self.len - at;
        let mut tail = Vec::with_capacity(count);
        // SAFETY: the items are moved, not copied: the source forgets them
        // by its count, the destination counts them in after the copy,
        // which lies in the capacity reserved.
        unsafe {
            std::ptr::copy_nonoverlapping(self.ptr.add(at), tail.as_mut_ptr(), count);
            self.len = at;
            tail.set_len(count);
        }
        tail
    }

    /// The items from `at` on, moved onto the end of `into` in order
    /// (the `drain` of a `Vec` into a buffer that is reused).
    pub fn drain_into(&mut self, at: usize, into: &mut Vec<T>) {
        assert!(at <= self.len, "the drain starts inside the vector");
        let count = self.len - at;
        into.reserve(count);
        // SAFETY: the items are moved, not copied: the source forgets them
        // by its count, the destination counts them in after the copy,
        // which lies in the capacity reserved.
        unsafe {
            std::ptr::copy_nonoverlapping(
                self.ptr.add(at),
                into.as_mut_ptr().add(into.len()),
                count,
            );
            self.len = at;
            into.set_len(into.len() + count);
        }
    }
}

impl<T: Clone> Pinned<T> {
    pub fn resize(&mut self, len: usize, value: T) {
        self.room(len);
        self.truncate(len);
        while self.len < len {
            // SAFETY: `len < cap`, so the slot is allocated and not in use.
            unsafe { self.ptr.add(self.len).write(value.clone()) };
            self.len += 1;
        }
    }
}

impl<T> Default for Pinned<T> {
    fn default() -> Pinned<T> {
        Pinned::new()
    }
}

impl<T> From<Pinned<T>> for Vec<T> {
    fn from(mut pinned: Pinned<T>) -> Vec<T> {
        pinned.take()
    }
}

impl<T> From<Vec<T>> for Pinned<T> {
    fn from(vec: Vec<T>) -> Pinned<T> {
        let mut pinned = Pinned {
            ptr: std::ptr::NonNull::dangling().as_ptr(),
            len: 0,
            cap: 0,
        };
        pinned.put(vec);
        pinned
    }
}

impl<T> Drop for Pinned<T> {
    fn drop(&mut self) {
        drop(self.take());
    }
}

impl<T> Deref for Pinned<T> {
    type Target = [T];

    #[inline]
    fn deref(&self) -> &[T] {
        // SAFETY: `len` items are in use at `ptr`.
        unsafe { std::slice::from_raw_parts(self.ptr, self.len) }
    }
}

impl<T> DerefMut for Pinned<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut [T] {
        // SAFETY: `len` items are in use at `ptr`, and this is the one
        // reference.
        unsafe { std::slice::from_raw_parts_mut(self.ptr, self.len) }
    }
}

impl<T: Clone> Clone for Pinned<T> {
    fn clone(&self) -> Pinned<T> {
        Pinned::from(self.to_vec())
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for Pinned<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        (**self).fmt(f)
    }
}

impl<T: PartialEq> PartialEq for Pinned<T> {
    fn eq(&self, other: &Pinned<T>) -> bool {
        **self == **other
    }
}

impl<T: Eq> Eq for Pinned<T> {}

impl<T: Hash> Hash for Pinned<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (**self).hash(state);
    }
}

impl<'a, T> IntoIterator for &'a Pinned<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::Pinned;

    #[test]
    fn the_vector_pushes_pops_and_moves_its_items() {
        let mut pinned: Pinned<String> = Pinned::new();
        for index in 0..10 {
            pinned.push(index.to_string());
        }
        assert_eq!(pinned.len(), 10);
        assert_eq!(pinned.pop().as_deref(), Some("9"));
        pinned.truncate(7);
        assert_eq!(pinned.last().map(String::as_str), Some("6"));
        pinned.insert(0, "first".to_string());
        assert_eq!(pinned.remove(1), "0");
        let mut tail = Vec::new();
        pinned.drain_into(5, &mut tail);
        assert_eq!(tail, vec!["5", "6"]);
        assert_eq!(pinned.len(), 5);
        pinned.resize(7, "x".to_string());
        assert_eq!(pinned.split_off(5), vec!["x", "x"]);
        pinned.extend(["y".to_string()]);
        assert_eq!(&pinned[..], &["first", "1", "2", "3", "4", "y"]);
        assert_eq!(pinned.clone(), pinned);
    }
}
