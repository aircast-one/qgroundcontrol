use std::ffi::c_void;
use std::ptr::NonNull;

pub struct Window {
    handle: NonNull<c_void>,
    #[cfg(target_os = "ios")]
    size: (u32, u32),
}

unsafe impl Send for Window {}

impl Window {
    pub fn key(&self) -> usize {
        self.handle.as_ptr() as usize
    }
}

#[cfg(target_os = "android")]
#[link(name = "android")]
unsafe extern "C" {
    fn ANativeWindow_fromSurface(
        env: *mut jni::sys::JNIEnv,
        surface: jni::sys::jobject,
    ) -> *mut c_void;
    fn ANativeWindow_release(window: *mut c_void);
    fn ANativeWindow_getWidth(window: *mut c_void) -> i32;
    fn ANativeWindow_getHeight(window: *mut c_void) -> i32;
}

#[cfg(target_os = "android")]
impl Window {
    pub unsafe fn from_surface(
        env: *mut jni::sys::JNIEnv,
        surface: jni::sys::jobject,
    ) -> Option<Self> {
        (!surface.is_null())
            .then(|| NonNull::new(unsafe { ANativeWindow_fromSurface(env, surface) }))
            .flatten()
            .map(|handle| Self { handle })
    }

    pub fn size(&self) -> (u32, u32) {
        let width = unsafe { ANativeWindow_getWidth(self.handle.as_ptr()) };
        let height = unsafe { ANativeWindow_getHeight(self.handle.as_ptr()) };
        (
            u32::try_from(width).unwrap_or(0),
            u32::try_from(height).unwrap_or(0),
        )
    }

    pub fn target(&self) -> wgpu::SurfaceTargetUnsafe {
        use wgpu::rwh::{
            AndroidDisplayHandle, AndroidNdkWindowHandle, RawDisplayHandle, RawWindowHandle,
        };
        wgpu::SurfaceTargetUnsafe::RawHandle {
            raw_display_handle: Some(RawDisplayHandle::Android(AndroidDisplayHandle::new())),
            raw_window_handle: RawWindowHandle::AndroidNdk(AndroidNdkWindowHandle::new(
                self.handle,
            )),
        }
    }
}

#[cfg(target_os = "android")]
impl Drop for Window {
    fn drop(&mut self) {
        unsafe { ANativeWindow_release(self.handle.as_ptr()) }
    }
}

#[cfg(target_os = "ios")]
#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFRetain(object: *const c_void) -> *const c_void;
    fn CFRelease(object: *const c_void);
}

#[cfg(target_os = "ios")]
impl Window {
    pub unsafe fn from_layer(layer: *mut c_void, size: (u32, u32)) -> Option<Self> {
        NonNull::new(layer).map(|handle| {
            unsafe { CFRetain(handle.as_ptr()) };
            Self { handle, size }
        })
    }

    pub fn size(&self) -> (u32, u32) {
        self.size
    }

    pub fn target(&self) -> wgpu::SurfaceTargetUnsafe {
        wgpu::SurfaceTargetUnsafe::CoreAnimationLayer(self.handle.as_ptr())
    }
}

#[cfg(target_os = "ios")]
impl Drop for Window {
    fn drop(&mut self) {
        unsafe { CFRelease(self.handle.as_ptr()) }
    }
}
