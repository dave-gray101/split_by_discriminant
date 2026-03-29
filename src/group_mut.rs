//! [`GroupMut`] — a mutable borrow of a single discriminant group.

use std::ops::Index;

/// A mutable borrow of a single discriminant group within a [`DiscriminantMap`].
///
/// `GroupMut` wraps `&mut [G]` and exposes only the operations that are safe
/// with respect to the discriminant invariant:
///
/// - Immutable element access via [`Index`] and [`iter`](Self::iter)
/// - Mutable iteration via [`iter_mut`](Self::iter_mut)
/// - In-place reordering via [`sort_by`](Self::sort_by),
///   [`sort_unstable_by`](Self::sort_unstable_by), and [`reverse`](Self::reverse)
/// - Length queries via [`len`](Self::len) and [`is_empty`](Self::is_empty)
/// - Immutable slice view via [`as_slice`](Self::as_slice)
///
/// Intentionally absent:
/// - `IndexMut` — prevents the `group[i] = wrong_variant(...)` assignment syntax
/// - `push`, `drain`, `retain`, `truncate`, `extend` — structural modifications
///   that could add items from a different bucket or grow the group
/// - `as_mut_slice` — prevents callers from escaping back to a raw `&mut [G]`
///
/// # Variant-safety limitation
///
/// `GroupMut` removes the most accidental mutation paths, but cannot provide a
/// *hard* guarantee that variant labels are preserved.  When `G = &mut T`,
/// `iter_mut()` yields `&mut G = &mut (&mut T)`.  Through double-dereference a
/// caller can still replace the variant of the underlying enum:
///
/// ```rust,ignore
/// for item in group.iter_mut() {
///     **item = E::B("wrong".into()); // compiles — GroupMut cannot prevent this
/// }
/// ```
///
/// **This is not considered a regression from v0.5.**  `GroupMut` eliminates the
/// easy accidental path (`group[i] = ...`) while being transparent about what it
/// cannot prevent.  For guaranteed field-only mutation use the `as_mut_*` /
/// `map_as_mut` family of methods, which return `&mut Field` rather than `&mut T`.
///
/// [`DiscriminantMap`]: crate::DiscriminantMap
pub struct GroupMut<'a, G> {
    pub(crate) inner: &'a mut [G],
}

impl<'a, G> GroupMut<'a, G> {
    pub(crate) fn new(inner: &'a mut [G]) -> Self {
        Self { inner }
    }

    /// Number of elements in this group.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns `true` if the group contains no elements.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Immutable view of the group as a shared slice.
    pub fn as_slice(&self) -> &[G] {
        self.inner
    }

    /// Iterator over shared references to each element.
    pub fn iter(&self) -> std::slice::Iter<'_, G> {
        self.inner.iter()
    }

    /// Iterator over mutable references to each element.
    ///
    /// # Variant-safety
    ///
    /// When `G = &mut T`, the items yielded are `&mut (&mut T)`.  Through
    /// double-dereference (`**item = ...`) a caller can still replace the
    /// variant of the underlying enum.  See the struct-level documentation for
    /// context and alternatives.
    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, G> {
        self.inner.iter_mut()
    }

    /// Sort the group in-place using a comparator.
    ///
    /// Reorders elements within this group only.  Does not move elements
    /// between groups or change any element's value.
    pub fn sort_by<F>(&mut self, compare: F)
    where
        F: FnMut(&G, &G) -> std::cmp::Ordering,
    {
        self.inner.sort_by(compare);
    }

    /// Unstable in-place sort using a comparator.
    pub fn sort_unstable_by<F>(&mut self, compare: F)
    where
        F: FnMut(&G, &G) -> std::cmp::Ordering,
    {
        self.inner.sort_unstable_by(compare);
    }

    /// Reverse the order of elements in the group in-place.
    pub fn reverse(&mut self) {
        self.inner.reverse();
    }
}

// ── Index (immutable — IndexMut intentionally absent) ─────────────────────────

impl<G> Index<usize> for GroupMut<'_, G> {
    type Output = G;
    fn index(&self, index: usize) -> &Self::Output {
        &self.inner[index]
    }
}

// ── IntoIterator ──────────────────────────────────────────────────────────────

/// Consuming iteration — yields `&'a mut G`, preserving the group's full lifetime.
impl<'a, G> IntoIterator for GroupMut<'a, G> {
    type Item = &'a mut G;
    type IntoIter = std::slice::IterMut<'a, G>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.iter_mut()
    }
}

/// Shared iteration over a borrowed `GroupMut`.
impl<'a, G> IntoIterator for &'a GroupMut<'_, G> {
    type Item = &'a G;
    type IntoIter = std::slice::Iter<'a, G>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.iter()
    }
}

/// Mutable iteration over a mutably-borrowed `GroupMut`.
impl<'a, G> IntoIterator for &'a mut GroupMut<'_, G> {
    type Item = &'a mut G;
    type IntoIter = std::slice::IterMut<'a, G>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.iter_mut()
    }
}
