package dev.vessel.reactnative;

import androidx.annotation.NonNull;
import com.facebook.react.uimanager.SimpleViewManager;
import com.facebook.react.uimanager.ThemedReactContext;
import com.facebook.react.uimanager.annotations.ReactProp;
import dev.vessel.android.VesselTextureView;

/** Exposes {@link VesselTextureView} to JavaScript as {@code VesselView}. */
public class VesselViewManager extends SimpleViewManager<VesselTextureView> {
  @NonNull
  @Override
  public String getName() {
    return "VesselView";
  }

  @NonNull
  @Override
  protected VesselTextureView createViewInstance(@NonNull ThemedReactContext context) {
    return new VesselTextureView(context);
  }

  @ReactProp(name = "surfaceId", defaultInt = -1)
  public void setSurfaceId(VesselTextureView view, int id) {
    view.setSurfaceId(id);
  }
}
