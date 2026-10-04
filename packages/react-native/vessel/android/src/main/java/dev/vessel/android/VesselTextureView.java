package dev.vessel.android;

import android.content.Context;
import android.graphics.SurfaceTexture;
import android.view.MotionEvent;
import android.view.Surface;
import android.view.TextureView;

/**
 * A view Vessel draws on directly from Rust. Its surface is registered under {@code surfaceId}. A TextureView (rather than
 * a SurfaceView) so the host UI's transforms, clipping and overlays keep working.
 */
public class VesselTextureView extends TextureView implements TextureView.SurfaceTextureListener {
  private int surfaceId = -1;
  private Surface surface;

  public VesselTextureView(Context context) {
    super(context);
    SurfaceBridge.load(context);
    setSurfaceTextureListener(this);
    setOpaque(false);
  }

  public void setSurfaceId(int id) {
    if (id == surfaceId) return;
    if (surface != null && surfaceId >= 0) SurfaceBridge.nativeSurfaceDestroyed(surfaceId);
    surfaceId = id;
    if (surface != null && surfaceId >= 0) {
      SurfaceBridge.nativeSurfaceAvailable(surfaceId, surface, getWidth(), getHeight());
    }
  }

  /**
   * Hands touches to Vessel, which gives them to the component shown here as pointer events. It does not stop a parent
   * from seeing the touch: React Native tracks every touch from the root, and hiding its end from it leaves it thinking a
   * finger is still down, which freezes every other button and gesture.
   */
  @Override
  public boolean onTouchEvent(MotionEvent event) {
    if (surfaceId < 0) return false;
    int action;
    switch (event.getActionMasked()) {
      case MotionEvent.ACTION_DOWN:
        action = 0;
        break;
      case MotionEvent.ACTION_MOVE:
        action = 1;
        break;
      case MotionEvent.ACTION_UP:
      case MotionEvent.ACTION_CANCEL:
        action = 2;
        break;
      default:
        return true;
    }
    SurfaceBridge.nativeTouch(surfaceId, action, event.getX(), event.getY());
    return true;
  }

  @Override
  public void onSurfaceTextureAvailable(SurfaceTexture texture, int width, int height) {
    surface = new Surface(texture);
    if (surfaceId >= 0) SurfaceBridge.nativeSurfaceAvailable(surfaceId, surface, width, height);
  }

  @Override
  public void onSurfaceTextureSizeChanged(SurfaceTexture texture, int width, int height) {
    if (surfaceId >= 0) SurfaceBridge.nativeSurfaceSizeChanged(surfaceId, width, height);
  }

  @Override
  public boolean onSurfaceTextureDestroyed(SurfaceTexture texture) {
    if (surfaceId >= 0) SurfaceBridge.nativeSurfaceDestroyed(surfaceId);
    if (surface != null) surface.release();
    surface = null;
    return true;
  }

  @Override
  public void onSurfaceTextureUpdated(SurfaceTexture texture) {}
}
