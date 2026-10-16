//! Android: рендер-поток на wgpu (Vulkan) + движок в фоне.

use crate::{config, run_engine, Runtime};
use anyhow::Result;
use std::sync::Arc;
use std::sync::atomic::Ordering;

pub fn run(rt: Option<Arc<Runtime>>, _startup: config::Startup, args: Vec<String>) -> Result<()> {
    log::info!("[app_android] run: старт");

    let rt = rt.expect("runtime");

    log::info!("[app_android] run: запускаю render-поток");
    std::thread::Builder::new()
        .name("RenderThread".to_string())
        .spawn(|| {
            log::info!("[render] поток стартовал");
            if let Err(e) = render_loop() {
                log::error!("Muguet: render_loop упал: {e:#}");
            }
        })?;
    log::info!("[app_android] run: render-поток запущен");

    log::info!("[app_android] run: запускаю движок");
    run_engine(&rt, args)?;

    log::info!("[app_android] run: движок завершился, парковка");
    loop {
        std::thread::park();
    }
}

fn render_loop() -> Result<()> {
    use crate::android_surface;

    log::info!("[render] ожидаю Surface от Kotlin...");

    loop {
        if android_surface::SURFACE_READY.load(Ordering::SeqCst) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    let window_ptr = android_surface::take_native_window()
        .ok_or_else(|| anyhow::anyhow!("native window is None"))?;
    let width = android_surface::SURFACE_WIDTH.load(Ordering::SeqCst);
    let height = android_surface::SURFACE_HEIGHT.load(Ordering::SeqCst);

    log::info!("[render] Surface готов: {width}x{height}, window={window_ptr:x}");

    // ============================================================
    // 1. wgpu Instance
    // Instance::new принимает InstanceDescriptor по значению (не по ссылке)
    // ============================================================
    let mut desc = wgpu::InstanceDescriptor::new_without_display_handle();
    desc.backends = wgpu::Backends::VULKAN;
    let instance = wgpu::Instance::new(desc);

    // ============================================================
    // 2. Surface
    // ============================================================
    let surface = unsafe {
        let raw_window_handle = raw_window_handle::RawWindowHandle::AndroidNdk(
            raw_window_handle::AndroidNdkWindowHandle::new(
                std::ptr::NonNull::new(window_ptr as *mut std::ffi::c_void).unwrap(),
            ),
        );
        let raw_display_handle = raw_window_handle::RawDisplayHandle::Android(
            raw_window_handle::AndroidDisplayHandle::new(),
        );

        instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::RawHandle {
            raw_window_handle,
            raw_display_handle: Some(raw_display_handle),
        })?
    };

    // ============================================================
    // 3. Adapter
    // ============================================================
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: Some(&surface),
        force_fallback_adapter: false,
        apply_limit_buckets: false,
    }))
    .map_err(|e| anyhow::anyhow!("request_adapter: {e:?}"))?;

    log::info!("[render] Адаптер: {:?}", adapter.get_info());

    // ============================================================
    // 4. Device + Queue
    // ============================================================
    let (device, queue) = pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("muguet-device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::downlevel_defaults(),
            memory_hints: wgpu::MemoryHints::default(),
            trace: wgpu::Trace::Off,
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
        },
    ))
    .map_err(|e| anyhow::anyhow!("request_device: {e:?}"))?;

    // ============================================================
    // 5. Surface configuration
    // ============================================================
    let caps = surface.get_capabilities(&adapter);
    let format = caps.formats.iter().copied().find(|f| f.is_srgb()).unwrap_or(caps.formats[0]);

    let config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format,
        width: width.max(1),
        height: height.max(1),
        present_mode: wgpu::PresentMode::Fifo,
        alpha_mode: caps.alpha_modes[0],
        view_formats: vec![],
        desired_maximum_frame_latency: 2,
        color_space: wgpu::SurfaceColorSpace::Auto,
    };

    surface.configure(&device, &config);

    log::info!("[render] wgpu готов, входим в render loop");

    let mut frame_count: u64 = 0;
    let start = std::time::Instant::now();

    // ============================================================
    // 6. Render loop
    // ============================================================
    loop {
        if android_surface::SURFACE_REMOVED.load(Ordering::SeqCst) {
            log::info!("[render] Surface удалён, выходим");
            return Ok(());
        }

        let output = match surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(tex) => tex,
            wgpu::CurrentSurfaceTexture::Suboptimal(tex) => {
                log::warn!("[render] suboptimal, reconfigure");
                surface.configure(&device, &config);
                tex
            }
            wgpu::CurrentSurfaceTexture::Timeout => {
                log::warn!("[render] timeout, ждём");
                std::thread::sleep(std::time::Duration::from_millis(16));
                continue;
            }
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                log::warn!("[render] surface outdated/lost, reconfigure");
                surface.configure(&device, &config);
                std::thread::sleep(std::time::Duration::from_millis(100));
                continue;
            }
            #[allow(unreachable_patterns)]
            _ => {
                log::error!("[render] unexpected surface state, skip frame");
                std::thread::sleep(std::time::Duration::from_millis(50));
                continue;
            }
        };

        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("clear-encoder"),
        });

        {
            let _rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("clear-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.05,
                            g: 0.05,
                            b: 0.15,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
        }

        queue.submit(Some(encoder.finish()));

        // В wgpu 30 SurfaceTexture презентится при drop() — явный present() не нужен
        drop(output);

        frame_count += 1;
        if frame_count % 60 == 0 {
            let fps = frame_count as f64 / start.elapsed().as_secs_f64();
            log::info!("[render] кадр {frame_count}, FPS ≈ {fps:.1}");
        }
    }
}