package com.alakazam.mobile

import com.facebook.react.BaseReactPackage
import com.facebook.react.bridge.NativeModule
import com.facebook.react.bridge.ReactApplicationContext
import com.facebook.react.module.model.ReactModuleInfo
import com.facebook.react.module.model.ReactModuleInfoProvider

class MediaFilePackage : BaseReactPackage() {
  override fun getModule(name: String, reactContext: ReactApplicationContext): NativeModule? =
    when (name) {
      MediaFileModule.NAME -> MediaFileModule(reactContext)
      ProjectStoreModule.NAME -> ProjectStoreModule(reactContext)
      else -> null
    }

  override fun getReactModuleInfoProvider(): ReactModuleInfoProvider =
    ReactModuleInfoProvider {
      mapOf(
        MediaFileModule.NAME to moduleInfo(MediaFileModule.NAME, MediaFileModule::class.java.name),
        ProjectStoreModule.NAME to moduleInfo(ProjectStoreModule.NAME, ProjectStoreModule::class.java.name),
      )
    }

  // Both are bridged through the New Architecture interop layer, so neither is a turbo or C++ module.
  private fun moduleInfo(name: String, className: String) =
    ReactModuleInfo(
      name,
      className,
      false, // canOverrideExistingModule
      false, // needsEagerInit
      false, // isCxxModule
      false, // isTurboModule
    )
}
