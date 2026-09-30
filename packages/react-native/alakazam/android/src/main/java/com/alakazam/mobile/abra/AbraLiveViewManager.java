package com.alakazam.mobile.abra;

import android.view.Surface;
import androidx.annotation.NonNull;
import com.facebook.react.uimanager.SimpleViewManager;
import com.facebook.react.uimanager.ThemedReactContext;
import com.facebook.react.uimanager.annotations.ReactProp;

/** Exposes {@link AbraLiveView} to JavaScript as {@code AbraLiveView}. */
public class AbraLiveViewManager extends SimpleViewManager<AbraLiveView> {
  static {
    // The Rust library implements the natives below and owns the surface registry.
    System.loadLibrary("alakazam_mobile");
  }

  static native void nativeSurfaceAvailable(int id, Surface surface, int width, int height);

  static native void nativeSurfaceSizeChanged(int id, int width, int height);

  static native void nativeSurfaceDestroyed(int id);

  @NonNull
  @Override
  public String getName() {
    return "AbraLiveView";
  }

  @NonNull
  @Override
  protected AbraLiveView createViewInstance(@NonNull ThemedReactContext context) {
    return new AbraLiveView(context);
  }

  @ReactProp(name = "surfaceId", defaultInt = -1)
  public void setSurfaceId(AbraLiveView view, int id) {
    view.setSurfaceId(id);
  }
}
