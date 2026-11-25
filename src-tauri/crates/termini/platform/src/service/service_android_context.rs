use summer::plugin::Service;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use termini_macros::base_app;
use crate::base::base_app::{AppContext, BaseApp};

#[base_app(sort = 90)]
#[derive(Clone, Service)]
pub struct AndroidContextService {}

#[async_trait]
impl BaseApp for AndroidContextService {
    fn name(&self) -> &'static str {
        "AndroidContextService"
    }

    async fn start(&self, _ctx: &AppContext) -> Result<()> {
        Self::bind().unwrap_or_else(|error| panic!("android context bind failed: {error:?}"));
        Ok(())
    }
}

impl AndroidContextService {
    fn bind() -> Result<()> {
        let ctx = tauri::tao::platform::android::prelude::main_android_context().ok_or_else(|| anyhow!("tao android context missing"))?;
        unsafe { ndk_context::initialize_android_context(ctx.java_vm, ctx.context_jobject) };
        Self::bind_rustls(ctx.java_vm, ctx.context_jobject)?;
        Ok(())
    }

    fn bind_rustls(java_vm: *mut std::ffi::c_void, context: *mut std::ffi::c_void) -> Result<()> {
        let vm = unsafe { jni22::JavaVM::from_raw(java_vm.cast()) };
        vm.attach_current_thread(|env| {
            let context = unsafe { jni22::objects::JObject::from_raw(env, context.cast()) };
            rustls_platform_verifier::android::init_with_env(env, context)
        }).map_err(|error| anyhow!("{error}"))
    }
}
