#import "VesselSurfaceBridge.h"

#include <dlfcn.h>

const VesselSurfaceBridge *VesselLoadSurfaceBridge(void) {
  static VesselSurfaceBridge bridge;
  static dispatch_once_t once;
  dispatch_once(&once, ^{
    NSString *name = [[NSBundle mainBundle] objectForInfoDictionaryKey:@"dev.vessel.library"];
    if (![name isKindOfClass:[NSString class]] || name.length == 0 || [name containsString:@"/"]) {
      [NSException raise:@"VesselConfiguration"
                  format:@"Set dev.vessel.library in the app's Info.plist to the Rust framework's library name."];
    }
    NSString *framework = [[[NSBundle mainBundle] privateFrameworksPath]
        stringByAppendingPathComponent:[name stringByAppendingString:@".framework"]];
    NSString *binary = [framework stringByAppendingPathComponent:name];
    // Keep the handle open: callbacks and running components belong to this same Rust library.
    void *library = dlopen(binary.fileSystemRepresentation, RTLD_NOW | RTLD_LOCAL);
    if (!library) {
      [NSException raise:@"VesselConfiguration" format:@"Could not load %@: %s", binary, dlerror()];
    }
    bridge.available = (typeof(bridge.available))dlsym(library, "vessel_ios_surface_available");
    bridge.resized = (typeof(bridge.resized))dlsym(library, "vessel_ios_surface_resized");
    bridge.destroyed = (typeof(bridge.destroyed))dlsym(library, "vessel_ios_surface_destroyed");
    bridge.touch = (typeof(bridge.touch))dlsym(library, "vessel_ios_touch");
    if (!bridge.available || !bridge.resized || !bridge.destroyed || !bridge.touch) {
      [NSException raise:@"VesselConfiguration"
                  format:@"%@ is missing Vessel's iOS bridge. Rebuild Rust with the ios-surface feature.", name];
    }
  });
  return &bridge;
}
