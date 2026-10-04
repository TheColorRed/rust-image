package dev.vessel.android;

import android.content.Context;
import android.content.pm.ApplicationInfo;
import android.content.pm.PackageManager;
import android.os.Bundle;
import android.view.Surface;

/**
 * The link between a view in the host app and Vessel's Rust library. A view reports its surface here, under an id, and the
 * engine finds the surface by that id. Any Android UI can use it: React Native, Flutter, or plain Java or Kotlin.
 *
 * <p>The app's native library is named in its manifest, so Vessel never hard-codes it:
 * {@code <meta-data android:name="dev.vessel.library" android:value="my_app_lib" />}
 */
public final class SurfaceBridge {
  private static boolean loaded;

  private SurfaceBridge() {}

  /** Loads the app's native library, named by the {@code dev.vessel.library} manifest entry. Safe to call many times. */
  public static synchronized void load(Context context) {
    if (loaded) return;
    try {
      ApplicationInfo info =
          context.getPackageManager().getApplicationInfo(context.getPackageName(), PackageManager.GET_META_DATA);
      Bundle data = info.metaData;
      String library = data == null ? null : data.getString("dev.vessel.library");
      if (library == null) {
        throw new IllegalStateException(
            "Add <meta-data android:name=\"dev.vessel.library\" android:value=\"your_lib\" /> to the manifest.");
      }
      System.loadLibrary(library);
      loaded = true;
    } catch (PackageManager.NameNotFoundException error) {
      throw new IllegalStateException(error);
    }
  }

  public static native void nativeSurfaceAvailable(int id, Surface surface, int width, int height);

  public static native void nativeSurfaceSizeChanged(int id, int width, int height);

  /** A touch on the view: {@code action} is 0 for down, 1 for moved and 2 for up or cancelled. */
  public static native void nativeTouch(int id, int action, float x, float y);

  public static native void nativeSurfaceDestroyed(int id);
}
