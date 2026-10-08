//! Constants shared across the alakazam package, kept in one place.

/// Where the catalog of downloadable AI models lives. Every other detail about a model comes from this file.
pub(crate) const MODEL_CATALOG_URL: &str = "https://alakazam-image-editor.s3.us-east-1.amazonaws.com/catalog.yml";

/// The role of the model the skin segmenter runs. The catalog says which model fills it.
pub(crate) const SKIN_SEGMENTATION_ROLE: &str = "skin-segmentation";
/// The role of the model the body finder runs. The catalog says which model fills it.
pub(crate) const PERSON_DETECTION_ROLE: &str = "person-detection";

/// How opaque the dark gradient behind a thumbnail's label is, from 0 to 1.
pub(crate) const THUMBNAIL_GRADIENT_OPACITY: f32 = 0.55;
