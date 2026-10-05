package dev.vessel.reactnative;

import androidx.annotation.NonNull;
import com.facebook.react.uimanager.SimpleViewManager;
import com.facebook.react.uimanager.ThemedReactContext;
import com.facebook.react.uimanager.annotations.ReactProp;
import dev.vessel.android.VesselTextureView;
import java.util.HashMap;
import java.util.Map;

/** Exposes {@link VesselTextureView} to JavaScript as {@code VesselView}. */
public class VesselViewManager extends SimpleViewManager<VesselTextureView> {
  @Override
  public Map<String, Object> getExportedViewConstants() {
    Map<String, Object> constants = new HashMap<>();
    // React Native's default Android text size, before density and accessibility scaling.
    constants.put("defaultFontSize", 14);
    return constants;
  }

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
