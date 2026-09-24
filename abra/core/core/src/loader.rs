/// Common operations available on collections of loaded resources.
pub trait LoadedCollection {
  /// The shared resource type stored by the collection.
  type Item: Clone;

  /// Adds a shared resource to the collection.
  fn add(&mut self, p_item: Self::Item) -> &mut Self;
  /// Removes and returns the first resource.
  fn shift(&mut self) -> Option<Self::Item>;
  /// Removes and returns the last resource.
  fn pop(&mut self) -> Option<Self::Item>;
  /// Removes a resource at `p_index` when it exists.
  fn drop(&mut self, p_index: usize) -> &mut Self;
  /// Gets a resource at `p_index`.
  fn at(&self, p_index: usize) -> Option<Self::Item>;
  /// Returns all resources.
  fn all(&self) -> Vec<Self::Item>;
  /// Returns the first resource.
  fn first(&self) -> Option<Self::Item>;
  /// Returns the last resource.
  fn last(&self) -> Option<Self::Item>;
}
