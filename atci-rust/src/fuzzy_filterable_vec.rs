use std::ops::{Deref, DerefMut, Index, IndexMut};
use std::slice::SliceIndex;
use std::fmt::Display;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FuzzyFilterableVec<T> {
    inner: Vec<T>,
}

impl<T> FuzzyFilterableVec<T> {
    pub fn new() -> Self {
        Self {
            inner: Vec::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            inner: Vec::with_capacity(capacity),
        }
    }

    pub fn from_vec(vec: Vec<T>) -> Self {
        Self { inner: vec }
    }

    pub fn filter_by_string(&self, search_string: &str) -> Vec<&T>
    where
        T: Display,
    {
        let search_lower = search_string.to_lowercase();
        self.inner
            .iter()
            .filter(|item| item.to_string().to_lowercase().contains(&search_lower))
            .collect()
    }

    pub fn push(&mut self, value: T) {
        self.inner.push(value);
    }

    pub fn pop(&mut self) -> Option<T> {
        self.inner.pop()
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn capacity(&self) -> usize {
        self.inner.capacity()
    }

    pub fn reserve(&mut self, additional: usize) {
        self.inner.reserve(additional);
    }

    pub fn shrink_to_fit(&mut self) {
        self.inner.shrink_to_fit();
    }

    pub fn insert(&mut self, index: usize, element: T) {
        self.inner.insert(index, element);
    }

    pub fn remove(&mut self, index: usize) -> T {
        self.inner.remove(index)
    }

    pub fn clear(&mut self) {
        self.inner.clear();
    }

    pub fn append(&mut self, other: &mut Vec<T>) {
        self.inner.append(other);
    }

    pub fn drain<R>(&mut self, range: R) -> std::vec::Drain<'_, T>
    where
        R: std::ops::RangeBounds<usize>,
    {
        self.inner.drain(range)
    }

    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.inner.iter()
    }

    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, T> {
        self.inner.iter_mut()
    }

    pub fn into_vec(self) -> Vec<T> {
        self.inner
    }

    pub fn as_slice(&self) -> &[T] {
        &self.inner
    }

    pub fn as_mut_slice(&mut self) -> &mut [T] {
        &mut self.inner
    }
}

impl<T> Default for FuzzyFilterableVec<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Deref for FuzzyFilterableVec<T> {
    type Target = Vec<T>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> DerefMut for FuzzyFilterableVec<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<T, I> Index<I> for FuzzyFilterableVec<T>
where
    I: SliceIndex<[T]>,
{
    type Output = I::Output;

    fn index(&self, index: I) -> &Self::Output {
        &self.inner[index]
    }
}

impl<T, I> IndexMut<I> for FuzzyFilterableVec<T>
where
    I: SliceIndex<[T]>,
{
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        &mut self.inner[index]
    }
}

impl<T> From<Vec<T>> for FuzzyFilterableVec<T> {
    fn from(vec: Vec<T>) -> Self {
        Self::from_vec(vec)
    }
}

impl<T> Into<Vec<T>> for FuzzyFilterableVec<T> {
    fn into(self) -> Vec<T> {
        self.inner
    }
}

impl<T> FromIterator<T> for FuzzyFilterableVec<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self {
            inner: Vec::from_iter(iter),
        }
    }
}

impl<T> IntoIterator for FuzzyFilterableVec<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

impl<'a, T> IntoIterator for &'a FuzzyFilterableVec<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.iter()
    }
}

impl<'a, T> IntoIterator for &'a mut FuzzyFilterableVec<T> {
    type Item = &'a mut T;
    type IntoIter = std::slice::IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.iter_mut()
    }
}