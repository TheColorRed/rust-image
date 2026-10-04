use std::sync::Arc;

use vessel_engine::GpuFrame;

/// The event that gives a component a picture made on the GPU, to be drawn without copying it to the CPU. Send it on
/// `component.subject::<GpuPicture>()`:
///
/// ```ignore
/// component.subject::<GpuPicture>().next(GpuPicture(Some(frame)));   // show this picture
/// component.subject::<GpuPicture>().next(GpuPicture(None));          // back to what the component draws on the CPU
/// ```
///
/// While a component has a GPU picture it is the whole picture of the component: its `draw()` listeners and children are
/// not used. A surface without a GPU target reads the picture back, so it is still shown, only slower.
#[derive(Clone)]
pub struct GpuPicture(pub Option<Arc<dyn GpuFrame>>);
