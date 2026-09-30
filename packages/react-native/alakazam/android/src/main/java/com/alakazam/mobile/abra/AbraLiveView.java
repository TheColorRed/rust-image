package com.alakazam.mobile.abra;

import android.content.Context;
import android.graphics.SurfaceTexture;
import android.view.Surface;
import android.view.TextureView;

/**
 * A view whose surface a live preview draws on directly from Rust. The surface is registered under {@code surfaceId};
 * JavaScript hands the same id to the preview. A TextureView (rather than a SurfaceView) so React Native's transforms,
 * clipping and overlays keep working.
 */
public class AbraLiveView extends TextureView implements TextureView.SurfaceTextureListener {
  private int surfaceId = -1;
  private Surface surface;

  public AbraLiveView(Context context) {
    super(context);
    setSurfaceTextureListener(this);
    setOpaque(false);
  }

  public void setSurfaceId(int id) {
    if (id == surfaceId) return;
    if (surface != null && surfaceId >= 0) AbraLiveViewManager.nativeSurfaceDestroyed(surfaceId);
    surfaceId = id;
    if (surface != null && surfaceId >= 0) {
      AbraLiveViewManager.nativeSurfaceAvailable(surfaceId, surface, getWidth(), getHeight());
    }
  }

  @Override
  public void onSurfaceTextureAvailable(SurfaceTexture texture, int width, int height) {
    surface = new Surface(texture);
    if (surfaceId >= 0) AbraLiveViewManager.nativeSurfaceAvailable(surfaceId, surface, width, height);
  }

  @Override
  public void onSurfaceTextureSizeChanged(SurfaceTexture texture, int width, int height) {
    if (surfaceId >= 0) AbraLiveViewManager.nativeSurfaceSizeChanged(surfaceId, width, height);
  }

  @Override
  public boolean onSurfaceTextureDestroyed(SurfaceTexture texture) {
    if (surfaceId >= 0) AbraLiveViewManager.nativeSurfaceDestroyed(surfaceId);
    if (surface != null) surface.release();
    surface = null;
    return true;
  }

  @Override
  public void onSurfaceTextureUpdated(SurfaceTexture texture) {}
}
