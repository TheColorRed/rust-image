/// Common operations available on collections of loaded resources, such as [`crate::LoadedImages`].
///
/// A collection only provides access to its storage ([`LoadedCollection::items`] and
/// [`LoadedCollection::items_mut`]); every other method has a shared default implementation.
pub trait LoadedCollection {
  /// The shared resource type stored by the collection.
  type Item: Clone;

  /// The stored resources.
  fn items(&self) -> &[Self::Item];
  /// The stored resources, for adding or removing.
  fn items_mut(&mut self) -> &mut Vec<Self::Item>;

  /// Adds a resource to the end of the collection.
  fn add(&mut self, p_item: impl Into<Self::Item>) -> &mut Self {
    self.items_mut().push(p_item.into());
    self
  }
  /// Removes and returns the first resource.
  fn shift(&mut self) -> Option<Self::Item> {
    if self.items().is_empty() { None } else { Some(self.items_mut().remove(0)) }
  }
  /// Removes and returns the last resource.
  fn pop(&mut self) -> Option<Self::Item> {
    self.items_mut().pop()
  }
  /// Removes the resource at `p_index` when it exists.
  fn remove(&mut self, p_index: usize) -> &mut Self {
    if p_index < self.items().len() {
      self.items_mut().remove(p_index);
    }
    self
  }
  /// Gets the resource at `p_index`.
  fn at(&self, p_index: usize) -> Option<Self::Item> {
    self.items().get(p_index).cloned()
  }
  /// Returns all resources.
  fn all(&self) -> Vec<Self::Item> {
    self.items().to_vec()
  }
  /// Returns the first resource.
  fn first(&self) -> Option<Self::Item> {
    self.items().first().cloned()
  }
  /// Returns the last resource.
  fn last(&self) -> Option<Self::Item> {
    self.items().last().cloned()
  }
  /// The number of resources.
  fn len(&self) -> usize {
    self.items().len()
  }
  /// Whether the collection is empty.
  fn is_empty(&self) -> bool {
    self.items().is_empty()
  }
}
