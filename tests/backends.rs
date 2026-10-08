//! alhazen-gpui's default build carries alhazen-core's default backends (the workspace dependency
//! turns alhazen-core's defaults off, so each one must be forwarded explicitly).

use alhazen_gpui::alhazen_core::backend::Registry;

#[test]
fn default_build_includes_ffmpeg_cli() {
    let names = Registry::with_ffmpeg(&Default::default()).names();
    assert!(names.contains(&"ffmpeg-cli"), "backends: {names:?}");
}

#[cfg(windows)]
#[test]
fn default_build_includes_media_foundation() {
    let names = Registry::with_ffmpeg(&Default::default()).names();
    assert!(names.contains(&"media-foundation"), "backends: {names:?}");
}

/// On a Linux machine with NVIDIA's driver, the default build registers NVDEC.
#[cfg(target_os = "linux")]
#[test]
fn default_build_includes_nvdec_when_the_driver_is_there() {
    // Asked independently of alhazen-core (whose `available()` is false when the feature is off).
    let ldconfig = std::process::Command::new("ldconfig").arg("-p").output().map(|o| o.stdout).unwrap_or_default();
    if !String::from_utf8_lossy(&ldconfig).contains("libnvcuvid.so.1") {
        eprintln!("skipped: no NVIDIA driver");
        return;
    }
    let names = Registry::with_ffmpeg(&Default::default()).names();
    assert!(names.contains(&"nvdec"), "backends: {names:?}");
}
