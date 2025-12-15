use napi_derive::napi;

#[napi(object)]
pub struct ProjectMetadata {
  /// A UUID string identifying the project.
  pub id: String,
  /// The name of the project.
  pub name: String,
  /// The width of the project in pixels.
  pub width: u32,
  /// The height of the project in pixels.
  pub height: u32,
  /// The layers that are currently active/selected.
  pub active_layers: Vec<LayerMetadata>,
  /// All layers in the project.
  pub layers: Vec<LayerMetadata>,
}

#[napi(object)]
pub struct LayerMetadata {
  /// A UUID string identifying the layer.
  pub id: String,
  /// The name of the layer.
  pub name: String,
  /// The id of the project this layer belongs to.
  pub project_id: String,
  /// The blend mode for the layer.
  pub blend_mode: String,
  /// The opacity of the layer (0.0 to 1.0).
  pub opacity: f64,
  /// Whether the layer is visible.
  pub visible: bool,
  /// The order of the layer in the layer stack (0 is bottom).
  pub order: u32,
  /// The adjustment type if this is an adjustment layer.
  pub adjustment_type: Option<String>,
  /// The width of the layer in pixels.
  pub width: u32,
  /// The height of the layer in pixels.
  pub height: u32,
  /// The x offset of the layer in pixels.
  pub x: i32,
  /// The y offset of the layer in pixels.
  pub y: i32,
  /// The rotation angle of the layer in degrees.
  pub angle: f64,
}
