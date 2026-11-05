//! Constants shared across the alakazam package, kept in one place.

/// The role of the model the skin segmenter runs. The app's catalog says which model fills it.
pub(crate) const SKIN_SEGMENTATION_ROLE: &str = "skin-segmentation";
/// The role of the model the body finder runs. The app's catalog says which model fills it.
pub(crate) const PERSON_DETECTION_ROLE: &str = "person-detection";
/// The role of the guided foreground-edge refinement model.
pub(crate) const EDGE_REFINEMENT_ROLE: &str = "edge-refinement-small";

/// How opaque the dark gradient behind a thumbnail's label is, from 0 to 1.
pub(crate) const THUMBNAIL_GRADIENT_OPACITY: f32 = 0.55;
