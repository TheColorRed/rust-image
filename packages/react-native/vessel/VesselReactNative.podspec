require "json"

package = JSON.parse(File.read(File.join(__dir__, "package.json")))

Pod::Spec.new do |s|
  s.name = "VesselReactNative"
  s.version = package["version"]
  s.summary = package["description"]
  s.homepage = "https://github.com/TheColorRed/rust-image"
  s.license = "UNLICENSED"
  s.authors = "Alakazam contributors"
  s.platforms = { :ios => min_ios_version_supported }
  s.source = { :git => "https://github.com/TheColorRed/rust-image.git", :tag => s.version.to_s }
  s.source_files = "ios/**/*.{h,m,mm}"
  s.frameworks = "CoreText", "Metal", "QuartzCore", "UIKit"
  s.requires_arc = true

  install_modules_dependencies(s)
end
