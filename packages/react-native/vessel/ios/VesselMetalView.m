#import "VesselSurfaceBridge.h"

#import <CoreText/CoreText.h>
#import <Metal/Metal.h>
#import <QuartzCore/CAMetalLayer.h>
#import <React/RCTView.h>
#import <React/RCTViewManager.h>

#include <math.h>

// Used only for pixel frames. GPU frames use Vessel's wgpu presenter on this same layer.
@interface VesselPixelRenderer : NSObject
@property(nonatomic, strong) CAMetalLayer *layer;
@property(nonatomic, strong) id<MTLDevice> device;
@property(nonatomic, strong) id<MTLCommandQueue> queue;
@property(nonatomic, strong) id<MTLRenderPipelineState> pipeline;
@property(nonatomic, strong) id<MTLTexture> texture;
- (BOOL)draw:(const uint8_t *)pixels width:(uint32_t)width height:(uint32_t)height
    targetWidth:(uint32_t)targetWidth targetHeight:(uint32_t)targetHeight;
@end

@implementation VesselPixelRenderer
- (BOOL)prepare {
  if (self.pipeline) return YES;
  self.device = MTLCreateSystemDefaultDevice();
  self.queue = [self.device newCommandQueue];
  NSError *error = nil;
  NSString *shader =
      @"#include <metal_stdlib>\n"
       "using namespace metal;\n"
       "struct Vertex { float4 position [[position]]; float2 uv; };\n"
       "vertex Vertex vessel_vertex(uint i [[vertex_id]]) {\n"
       "  float2 p = float2((i << 1u) & 2u, i & 2u);\n"
       "  return {float4(p * 2.0 - 1.0, 0.0, 1.0), float2(p.x, 1.0 - p.y)};\n"
       "}\n"
       "fragment float4 vessel_fragment(Vertex v [[stage_in]], texture2d<float> image [[texture(0)]]) {\n"
       "  constexpr sampler s(coord::normalized, address::clamp_to_edge, filter::linear);\n"
       "  float4 color = image.sample(s, v.uv);\n"
       "  return float4(color.rgb * color.a, color.a);\n"
       "}\n";
  id<MTLLibrary> library = [self.device newLibraryWithSource:shader options:nil error:&error];
  MTLRenderPipelineDescriptor *descriptor = [MTLRenderPipelineDescriptor new];
  descriptor.vertexFunction = [library newFunctionWithName:@"vessel_vertex"];
  descriptor.fragmentFunction = [library newFunctionWithName:@"vessel_fragment"];
  descriptor.colorAttachments[0].pixelFormat = MTLPixelFormatBGRA8Unorm;
  if (descriptor.vertexFunction && descriptor.fragmentFunction) {
    self.pipeline = [self.device newRenderPipelineStateWithDescriptor:descriptor error:&error];
  }
  if (!self.queue || !self.pipeline) {
    NSLog(@"Vessel: could not create the Metal pixel renderer: %@", error);
    return NO;
  }
  return YES;
}

- (BOOL)draw:(const uint8_t *)pixels width:(uint32_t)width height:(uint32_t)height
    targetWidth:(uint32_t)targetWidth targetHeight:(uint32_t)targetHeight {
  if (![self prepare]) return NO;
  if (!self.texture || self.texture.width != width || self.texture.height != height) {
    MTLTextureDescriptor *descriptor =
        [MTLTextureDescriptor texture2DDescriptorWithPixelFormat:MTLPixelFormatRGBA8Unorm
                                                        width:width height:height mipmapped:NO];
    descriptor.usage = MTLTextureUsageShaderRead;
    descriptor.storageMode = MTLStorageModeShared;
    self.texture = [self.device newTextureWithDescriptor:descriptor];
  }
  if (!self.texture) {
    NSLog(@"Vessel: could not allocate the pixel texture");
    return NO;
  }
  self.layer.device = self.device;
  self.layer.pixelFormat = MTLPixelFormatBGRA8Unorm;
  self.layer.drawableSize = CGSizeMake(targetWidth, targetHeight);
  id<CAMetalDrawable> drawable = [self.layer nextDrawable];
  if (!drawable) return NO;
  // A new upload cannot overwrite a texture still used by the preceding frame.
  id<MTLCommandBuffer> command = [self.queue commandBuffer];
  if (!command) {
    NSLog(@"Vessel: could not allocate a Metal command buffer");
    return NO;
  }
  id<MTLTexture> texture = self.texture;
  [texture replaceRegion:MTLRegionMake2D(0, 0, width, height) mipmapLevel:0
              withBytes:pixels bytesPerRow:(NSUInteger)width * 4];
  MTLRenderPassDescriptor *pass = [MTLRenderPassDescriptor renderPassDescriptor];
  pass.colorAttachments[0].texture = drawable.texture;
  pass.colorAttachments[0].loadAction = MTLLoadActionClear;
  pass.colorAttachments[0].storeAction = MTLStoreActionStore;
  pass.colorAttachments[0].clearColor = MTLClearColorMake(0, 0, 0, 0);
  id<MTLRenderCommandEncoder> encoder = [command renderCommandEncoderWithDescriptor:pass];
  if (!encoder) {
    NSLog(@"Vessel: could not allocate a Metal render encoder");
    return NO;
  }
  [encoder setRenderPipelineState:self.pipeline];
  [encoder setFragmentTexture:texture atIndex:0];
  [encoder drawPrimitives:MTLPrimitiveTypeTriangle vertexStart:0 vertexCount:3];
  [encoder endEncoding];
  [command presentDrawable:drawable];
  [command commit];
  [command waitUntilCompleted];
  if (command.status == MTLCommandBufferStatusError) {
    NSLog(@"Vessel: Metal pixel presentation failed: %@", command.error);
    return NO;
  }
  return YES;
}
@end

static bool VesselDraw(void *context, uint32_t width, uint32_t height, const uint8_t *pixels,
                       size_t length, uint32_t targetWidth, uint32_t targetHeight) {
  @autoreleasepool {
    return [(__bridge VesselPixelRenderer *)context draw:pixels width:width height:height
                                            targetWidth:targetWidth targetHeight:targetHeight];
  }
}

static void VesselRelease(void *context) {
  @autoreleasepool {
    __unused VesselPixelRenderer *renderer = CFBridgingRelease(context);
  }
}

@interface VesselMetalView : RCTView
@property(nonatomic, assign) int32_t surfaceId;
@end

@implementation VesselMetalView {
  const VesselSurfaceBridge *_bridge;
  BOOL _registered;
  uint32_t _pixelWidth;
  uint32_t _pixelHeight;
  UITouch *_activeTouch;
}

+ (Class)layerClass {
  return [CAMetalLayer class];
}

- (instancetype)initWithFrame:(CGRect)frame {
  if ((self = [super initWithFrame:frame])) {
    _surfaceId = -1;
    _bridge = VesselLoadSurfaceBridge();
    self.opaque = NO;
    CAMetalLayer *layer = (CAMetalLayer *)self.layer;
    layer.opaque = NO;
    layer.framebufferOnly = YES;
    layer.presentsWithTransaction = NO;
    CGColorSpaceRef colorSpace = CGColorSpaceCreateWithName(kCGColorSpaceSRGB);
    layer.colorspace = colorSpace;
    CGColorSpaceRelease(colorSpace);
    [[NSNotificationCenter defaultCenter] addObserver:self selector:@selector(applicationWillResignActive:)
                                                 name:UIApplicationWillResignActiveNotification object:nil];
    [[NSNotificationCenter defaultCenter] addObserver:self selector:@selector(applicationDidBecomeActive:)
                                                 name:UIApplicationDidBecomeActiveNotification object:nil];
  }
  return self;
}

- (void)setSurfaceId:(int32_t)surfaceId {
  if (_surfaceId == surfaceId) return;
  [self retireSurface];
  _surfaceId = surfaceId;
  [self updateSurface];
}

- (void)didMoveToWindow {
  [super didMoveToWindow];
  [self updateSurface];
}

- (void)layoutSubviews {
  [super layoutSubviews];
  [self updateSurface];
}

- (void)applicationWillResignActive:(NSNotification *)notification {
  [self retireSurface];
}

- (void)applicationDidBecomeActive:(NSNotification *)notification {
  [self updateSurface];
}

- (void)updateSurface {
  CGFloat scale = self.window.screen.scale;
  uint32_t width = (uint32_t)llround(self.bounds.size.width * scale);
  uint32_t height = (uint32_t)llround(self.bounds.size.height * scale);
  if (!self.window || UIApplication.sharedApplication.applicationState != UIApplicationStateActive ||
      _surfaceId < 0 || width == 0 || height == 0) {
    [self retireSurface];
    return;
  }
  self.contentScaleFactor = scale;
  self.layer.contentsScale = scale;
  if (!_registered) {
    VesselPixelRenderer *renderer = [VesselPixelRenderer new];
    renderer.layer = (CAMetalLayer *)self.layer;
    void *context = (void *)CFBridgingRetain(renderer);
    _registered = _bridge->available(_surfaceId, (__bridge void *)self.layer, context,
                                    VesselDraw, VesselRelease, width, height);
    if (!_registered) {
      VesselRelease(context);
      [NSException raise:@"VesselSurface" format:@"Could not register iOS surface %d", _surfaceId];
    }
  } else if (_pixelWidth != width || _pixelHeight != height) {
    _bridge->resized(_surfaceId, width, height);
  }
  _pixelWidth = width;
  _pixelHeight = height;
}

- (void)retireSurface {
  if (!_registered) return;
  if (_activeTouch) [self forwardTouch:_activeTouch action:2];
  _activeTouch = nil;
  _bridge->destroyed(_surfaceId);
  _registered = NO;
  _pixelWidth = _pixelHeight = 0;
}

- (void)forwardTouch:(UITouch *)touch action:(int32_t)action {
  if (!_registered) return;
  CGPoint point = [touch locationInView:self];
  CGFloat scale = self.contentScaleFactor;
  _bridge->touch(_surfaceId, action, point.x * scale, point.y * scale);
}

- (void)touchesBegan:(NSSet<UITouch *> *)touches withEvent:(UIEvent *)event {
  if (!_activeTouch) {
    _activeTouch = touches.anyObject;
    [self forwardTouch:_activeTouch action:0];
  }
  [super touchesBegan:touches withEvent:event];
}

- (void)touchesMoved:(NSSet<UITouch *> *)touches withEvent:(UIEvent *)event {
  if (_activeTouch && [touches containsObject:_activeTouch]) [self forwardTouch:_activeTouch action:1];
  [super touchesMoved:touches withEvent:event];
}

- (void)touchesEnded:(NSSet<UITouch *> *)touches withEvent:(UIEvent *)event {
  if (_activeTouch && [touches containsObject:_activeTouch]) {
    [self forwardTouch:_activeTouch action:2];
    _activeTouch = nil;
  }
  [super touchesEnded:touches withEvent:event];
}

- (void)touchesCancelled:(NSSet<UITouch *> *)touches withEvent:(UIEvent *)event {
  if (_activeTouch && [touches containsObject:_activeTouch]) {
    [self forwardTouch:_activeTouch action:2];
    _activeTouch = nil;
  }
  [super touchesCancelled:touches withEvent:event];
}

- (void)dealloc {
  [[NSNotificationCenter defaultCenter] removeObserver:self];
  [self retireSurface];
}
@end

@interface VesselViewManager : RCTViewManager
@end

@implementation VesselViewManager
RCT_EXPORT_MODULE(VesselView)
RCT_EXPORT_VIEW_PROPERTY(surfaceId, int)

+ (BOOL)requiresMainQueueSetup {
  return YES;
}

- (NSDictionary *)constantsToExport {
  UIFont *font = [UIFont systemFontOfSize:[UIFont labelFontSize]];
  NSMutableDictionary *constants = [@{ @"defaultFontSize": @(font.pointSize) } mutableCopy];
  CTFontRef face = CTFontCreateWithName((__bridge CFStringRef)font.fontName, font.pointSize, NULL);
  CFTypeRef attribute = face ? CTFontCopyAttribute(face, kCTFontURLAttribute) : NULL;
  if (attribute && CFGetTypeID(attribute) == CFURLGetTypeID()) {
    NSURL *url = (__bridge NSURL *)attribute;
    if (url.isFileURL && [[NSFileManager defaultManager] isReadableFileAtPath:url.path]) {
      constants[@"defaultFontPath"] = url.path;
    }
  }
  if (attribute) CFRelease(attribute);
  if (face) CFRelease(face);
  if (!constants[@"defaultFontPath"]) {
    NSLog(@"Vessel: the system font file is unavailable; supply fontPath for an outline font. Using the fallback face.");
  }
  return constants;
}

- (UIView *)view {
  return [[VesselMetalView alloc] initWithFrame:CGRectZero];
}
@end
