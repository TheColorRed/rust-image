#import <Foundation/Foundation.h>

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

typedef bool (*VesselDrawPixels)(void *, uint32_t, uint32_t, const uint8_t *, size_t, uint32_t, uint32_t);
typedef void (*VesselReleaseSurface)(void *);

typedef struct {
  bool (*available)(int32_t, void *, void *, VesselDrawPixels, VesselReleaseSurface, uint32_t, uint32_t);
  void (*resized)(int32_t, uint32_t, uint32_t);
  void (*destroyed)(int32_t);
  void (*touch)(int32_t, int32_t, float, float);
} VesselSurfaceBridge;

FOUNDATION_EXPORT const VesselSurfaceBridge *VesselLoadSurfaceBridge(void);
