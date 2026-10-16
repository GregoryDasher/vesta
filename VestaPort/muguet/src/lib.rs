// muguet - lib.rs (движок, кросс-платформенный)
// Constantin Clerc (c22dev) - 0.1

#[cfg(not(target_os = "android"))]
mod audio;

#[cfg(target_os = "android")]
mod audio {
    //! Android-заглушка: звук пока не реализован
    pub fn start(_ptr: u64, _channels: u32) {}
}
mod config;
mod fs;

#[cfg(target_os = "android")]
mod gpu {
    //! Android-заглушка: графика пока не реализована
    use std::sync::atomic::{AtomicBool, AtomicU64};
    use wasmtime::SharedMemory;

    pub static PRESENTS: AtomicU64 = AtomicU64::new(0);
    pub static SUSPENDED: AtomicBool = AtomicBool::new(false);

    pub fn start(_mem: &SharedMemory, _a: u64, _b: u64, _c: u64) {
        // no-op
    }

    pub fn max_msaa() -> u32 {
        // На Android пока нет графики — возвращаем минимальное значение
        1
    }
}
mod host;
mod heapcheck;
mod import;
mod input;
mod io;
mod patch;

#[cfg(target_os = "ios")]
mod mp;
#[cfg(target_os = "ios")]
mod proto;
#[cfg(target_os = "ios")]
mod server;
#[cfg(target_os = "ios")]
mod opus;

// iOS-специфичные модули
#[cfg(target_os = "ios")]
mod app;
#[cfg(target_os = "ios")]
mod keyboard;
#[cfg(target_os = "ios")]
mod mouse;
#[cfg(target_os = "ios")]
mod pad;
#[cfg(target_os = "ios")]
mod touch;

// Android-специфичные модули (заглушки, потом заменим на реальные)
#[cfg(target_os = "android")]
mod app_android;
#[cfg(target_os = "android")]
mod pad_android;

// Десктоп-заглушка (Windows/macOS/Linux)
#[cfg(not(any(target_os = "ios", target_os = "android")))]
mod app_desktop;

use anyhow::{Context, Result};
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, OnceLock};
use std::time::Instant;
use wasmtime::*;

pub struct ImportSpec {
    pub module: String,
    pub name: String,
    pub ty: FuncType,
}

pub struct Runtime {
    pub engine: Engine,
    pub module: Module,
    pub mem: SharedMemory,
    pub imports: Vec<ImportSpec>,
    pub counts: Vec<AtomicU64>,
    pub start: Instant,
}

pub struct HostState {
    pub pthread: u64,
}

pub static RT: OnceLock<Arc<Runtime>> = OnceLock::new();

// ============================================================
// QoS (только для iOS/macOS)
// ============================================================
#[cfg(any(target_os = "ios", target_os = "macos"))]
pub fn set_interactive_qos() {
    const QOS_CLASS_USER_INTERACTIVE: u32 = 0x21;
    extern "C" {
        fn pthread_set_qos_class_self_np(qos: u32, relpri: i32) -> i32;
    }
    unsafe { pthread_set_qos_class_self_np(QOS_CLASS_USER_INTERACTIVE, 0) };
}

#[cfg(not(any(target_os = "ios", target_os = "macos")))]
pub fn set_interactive_qos() {
    // На Android/Windows QoS не нужен — приоритеты потоков настраиваются иначе
}

// ============================================================
// Лимит открытых файлов (только Unix)
// ============================================================
#[cfg(all(unix, not(target_os = "android")))]
fn raise_open_file_limit() {
    #[repr(C)]
    struct Rlimit { cur: u64, max: u64 }
    extern "C" {
        fn getrlimit(resource: i32, rlp: *mut Rlimit) -> i32;
        fn setrlimit(resource: i32, rlp: *const Rlimit) -> i32;
    }
    const RLIMIT_NOFILE: i32 = 8;
    let mut r = Rlimit { cur: 0, max: 0 };
    unsafe {
        if getrlimit(RLIMIT_NOFILE, &mut r) == 0 {
            let want = r.max.min(10240);
            if want > r.cur {
                r.cur = want;
                setrlimit(RLIMIT_NOFILE, &r);
            }
        }
    }
}

#[cfg(any(not(unix), target_os = "android"))]
fn raise_open_file_limit() {
    // На Android не трогаем — Android сам управляет лимитами,
    // вызов setrlimit может заблокироваться из-за seccomp.
}
// ============================================================
// Основная точка входа — вызывается из desktop-бинаря и из JNI
// ============================================================
pub fn run_muguet() -> Result<()> {
    log::info!("[run] step 1: raise_open_file_limit");
    raise_open_file_limit();
    log::info!("[run] step 2: config::setup");

    let startup = match config::setup() {
        Ok(s) => s,
        Err(e) => {
            log::error!("[run] config::setup упал: {e:#}");
            return Err(e);
        }
    };
    log::info!("[run] step 3: config::setup OK");

    log::info!("[run] step 4: make_runtime");
    let rt = Some(make_runtime(&startup)?);
    log::info!("[run] step 5: make_runtime OK");

    let mut args: Vec<String> = [
        "./this.program", "-rootdir=/game/", "-nodisplaycalibration", "-noSocialClub",
        "-pc:nosocialclub", "-nonetwork", "-output",
        "-forceResolution", "-Script_all=warning", "-replay_all=warning",
        "-fastscriptcore",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();
    args.extend(startup.engine_args.clone());
    args.extend(std::env::args().skip(1).filter(|a| !a.starts_with("-psn_")));

    #[cfg(target_os = "ios")]
    return app::run(rt, startup, args);

    #[cfg(target_os = "android")]
    {
        log::info!("[run] step 6: app_android::run");
        return app_android::run(rt, startup, args);
    }

    #[cfg(not(any(target_os = "ios", target_os = "android")))]
    return app_desktop::run(rt, startup, args);
}
// ============================================================
// Runtime
// ============================================================
pub fn make_runtime(startup: &config::Startup) -> Result<Arc<Runtime>> {
    report_progress(5, "Создаю engine...");
    let engine = Engine::new(&engine_config()).map_err(anyhow::Error::from).context("engine")?;

    let module = load_module(&engine, startup)?;

    report_progress(90, "Инициализирую runtime...");
    let mut mem = None;
    let mut imports = Vec::new();
    for imp in module.imports() {
        match imp.ty() {
            ExternType::Memory(mt) => mem = Some(SharedMemory::new(&engine, heap_type(mt)?)?),
            ExternType::Func(ty) => imports.push(ImportSpec {
                module: imp.module().to_string(),
                name: imp.name().to_string(),
                ty,
            }),
            other => anyhow::bail!("unsupported import {}.{}: {other:?}", imp.module(), imp.name()),
        }
    }
    let counts = imports.iter().map(|_| AtomicU64::new(0)).collect();
    let rt = Arc::new(Runtime {
        engine,
        module,
        mem: mem.context("module must import env.memory")?,
        imports,
        counts,
        start: Instant::now(),
    });
    RT.set(rt.clone()).ok();
    report_progress(95, "Runtime готов");
    Ok(rt)
}

fn heap_type(mt: MemoryType) -> Result<MemoryType> {
    Ok(mt)
}

pub fn run_engine(rt: &Arc<Runtime>, args: Vec<String>) -> Result<()> {
    report_progress(96, "Инициализация движка...");
    set_interactive_qos();

    log::info!("[host] instantiate...");
    let (mut store, instance) = match instantiate(rt, 0) {
        Ok(x) => x,
        Err(e) => {
            report_error(&format!("instantiate: {e:#}"));
            return Err(e);
        }
    };
    log::info!("[host] instantiate OK");
    report_progress(97, "Инстанс создан");

    log::info!("[host] _emscripten_tls_init...");
    if let Err(e) = call_export(&mut store, &instance, "_emscripten_tls_init", &[]) {
        report_error(&format!("tls_init: {e:#}"));
        return Err(e);
    }
    log::info!("[host] _emscripten_tls_init OK");
    report_progress(98, "TLS инициализирован");

    log::info!("[host] __wasm_call_ctors...");
    if let Err(e) = call_export(&mut store, &instance, "__wasm_call_ctors", &[]) {
        report_error(&format!("ctors: {e:#}"));
        return Err(e);
    }
    log::info!("[host] __wasm_call_ctors OK");
    report_progress(99, "Конструкторы выполнены");

    let argv = build_argv(&mut store, &instance, &rt.mem, &args)?;
    log::info!("[host] main: {}", args.join(" "));

    let main = instance.get_func(&mut store, "__main_argc_argv").context("no main")?;

    report_done("Запускаю главный цикл...");
    log::info!("[host] вызываю __main_argc_argv...");

    let mut out = [Val::I32(0)];
    match main.call(&mut store, &[Val::I32(args.len() as i32), Val::I64(argv as i64)], &mut out) {
        Ok(()) => {
            log::info!("[host] main returned {:?}", out[0]);
            report_done("Движок завершён");
        }
        Err(_) if host::take_unwind() => {
            log::info!("[host] main unwound; waiting on engine threads");
            report_done("Главный цикл запущен");
            loop { std::thread::park(); }
        }
        Err(e) => {
            log::error!("[host] main trapped: {e:?}");
            report_error(&format!("main: {e:?}"));
        }
    }
    Ok(())
}

// ============================================================
// Загрузка модуля
// ============================================================
fn load_module(engine: &Engine, startup: &config::Startup) -> Result<Module> {
    report_progress(10, "Читаю game.wasm...");
    let wasm_path = startup.wasm_path.clone();
    let wasm = std::fs::read(&wasm_path)
        .with_context(|| format!("{}", wasm_path.display()))?;
    report_progress(20, &format!("Прочитано {} МБ", wasm.len() >> 20));

    report_progress(25, "Патчу был (это займёт минуту)...");
    let patched = if patch::is_patched(&wasm) { wasm } else { patch::patch(&wasm, &[])? };
    report_progress(35, &format!("Патч применён ({} МБ)", patched.len() >> 20));

    report_progress(40, "Компилирую wasm в нативный код (1-3 минуты)...");
    let m = Module::new(engine, &patched).map_err(anyhow::Error::from).context("compile module")?;
    report_progress(85, "Компиляция завершена");
    Ok(m)
}

fn engine_config() -> Config {
    let mut config = Config::new();
    config.wasm_memory64(true);
    config.wasm_threads(true);
    config.shared_memory(true);
    config.wasm_bulk_memory(true);
    config.wasm_multi_memory(true);

    #[cfg(feature = "jit")]
    config.cranelift_opt_level(OptLevel::Speed);

    config.memory_reservation(17 << 30);
    config.memory_guard_size(2 << 30);
    config.memory_may_move(false);

    #[cfg(feature = "jit")]
    unsafe {
        config.cranelift_flag_set("enable_heap_access_spectre_mitigation", "false");
        config.cranelift_flag_set("enable_table_access_spectre_mitigation", "false");
    }
    config
}

// ============================================================
// Instantiate / pthreads
// ============================================================
pub fn instantiate(rt: &Arc<Runtime>, pthread: u64) -> Result<(Store<HostState>, Instance)> {
    let mut store = Store::new(&rt.engine, HostState { pthread });
    let mut linker: Linker<HostState> = Linker::new(&rt.engine);
    linker.define(&store, "env", "memory", rt.mem.clone())?;
    for (idx, imp) in rt.imports.iter().enumerate() {
        linker.func_new(&imp.module, &imp.name, imp.ty.clone(), move |caller, params, results| {
            host::call(caller, idx, params, results).map_err(wasmtime::Error::from_anyhow)
        })?;
    }
    let instance = linker.instantiate(&mut store, &rt.module)?;
    Ok((store, instance))
}

pub fn spawn_pthread(pthread_ptr: u64, start_routine: u64, arg: u64) -> i32 {
    let rt = RT.get().expect("runtime").clone();
    let spawned = std::thread::Builder::new()
        .name(format!("pthread-{pthread_ptr:x}"))
        .stack_size(16 << 20)
        .spawn(move || {
            if let Err(e) = run_pthread(&rt, pthread_ptr, start_routine, arg) {
                eprintln!("[host] pthread 0x{pthread_ptr:x} failed: {e:?}");
            }
        });
    if spawned.is_ok() { 0 } else { 6 }
}

fn run_pthread(rt: &Arc<Runtime>, ptr: u64, start_routine: u64, arg: u64) -> Result<()> {
    let (mut store, inst) = instantiate(rt, ptr)?;
    let high = read_u64(&rt.mem, ptr + 80).unwrap_or(0);
    let size = read_u64(&rt.mem, ptr + 88).unwrap_or(0);
    call_export(&mut store, &inst, "emscripten_stack_set_limits", &[Val::I64(high as i64), Val::I64((high - size) as i64)])?;
    call_export(&mut store, &inst, "_emscripten_stack_restore", &[Val::I64(high as i64)])?;
    call_export(&mut store, &inst, "_emscripten_thread_init", &[Val::I64(ptr as i64), Val::I32(0), Val::I32(0), Val::I32(1), Val::I64(0), Val::I32(0)])?;
    call_export(&mut store, &inst, "_emscripten_tls_init", &[])?;
    host::mailbox_await(&rt.mem, ptr);

    let table = inst.get_table(&mut store, "__indirect_function_table").context("no table")?;
    let entry = table.get(&mut store, start_routine)
        .and_then(|r| r.as_func().flatten().cloned())
        .with_context(|| format!("null start routine {start_routine}"))?;
    let mut res = [Val::I64(0)];
    let result = match entry.call(&mut store, &[Val::I64(arg as i64)], &mut res) {
        Ok(()) => match res[0] { Val::I64(v) => v, Val::I32(v) => v as i64, _ => 0 },
        Err(_) if host::take_unwind() => return Ok(()),
        Err(e) => { eprintln!("[host] thread trapped: {e:?}"); return Ok(()); }
    };
    match call_export(&mut store, &inst, "_emscripten_thread_exit", &[Val::I64(result)]) {
        Err(e) if !host::take_unwind() => Err(e.into()),
        _ => Ok(()),
    }
}

// ============================================================
// Общие хелперы
// ============================================================
pub fn call_export(store: &mut Store<HostState>, inst: &Instance, name: &str, args: &[Val]) -> Result<Vec<Val>> {
    let f = inst.get_func(&mut *store, name).with_context(|| format!("no export {name}"))?;
    let ty = f.ty(&mut *store);
    let args = coerce_args(args, &ty);
    let mut results: Vec<Val> = ty.results().map(|t| default_val(&t)).collect();
    f.call(&mut *store, &args, &mut results)?;
    Ok(results)
}

pub fn coerce_args(args: &[Val], ty: &FuncType) -> Vec<Val> {
    ty.params().zip(args).map(|(pt, v)| {
        let i = match v { Val::I32(x) => *x as i64, Val::I64(x) => *x, other => return other.clone() };
        match pt { ValType::I32 => Val::I32(i as i32), ValType::I64 => Val::I64(i), _ => v.clone() }
    }).collect()
}

fn build_argv(store: &mut Store<HostState>, inst: &Instance, mem: &SharedMemory, args: &[String]) -> Result<u64> {
    let alloc = inst.get_typed_func::<i64, i64>(&mut *store, "_emscripten_stack_alloc")?;
    let mut ptrs = Vec::new();
    for a in args {
        let bytes = [a.as_bytes(), &[0]].concat();
        let p = alloc.call(&mut *store, bytes.len() as i64)? as u64;
        write_bytes(mem, p, &bytes);
        ptrs.push(p);
    }
    let argv = alloc.call(&mut *store, ((ptrs.len() + 1) * 8) as i64)? as u64;
    for (i, p) in ptrs.iter().chain(std::iter::once(&0)).enumerate() {
        write_u64(mem, argv + i as u64 * 8, *p);
    }
    Ok(argv)
}

pub fn default_val(t: &ValType) -> Val {
    match t {
        ValType::I32 => Val::I32(0), ValType::I64 => Val::I64(0),
        ValType::F32 => Val::F32(0), ValType::F64 => Val::F64(0),
        ValType::V128 => Val::V128(0u128.into()),
        ValType::Ref(_) => Val::null_any_ref(),
    }
}

pub fn mem_base_len(mem: &SharedMemory) -> (*mut u8, usize) {
    let d = mem.data();
    (d.as_ptr() as *mut u8, d.len())
}
pub fn read_u64(mem: &SharedMemory, addr: u64) -> Option<u64> {
    let (base, len) = mem_base_len(mem); let a = addr as usize;
    if a + 8 > len { return None; }
    unsafe { Some(std::ptr::read_unaligned(base.add(a) as *const u64)) }
}
pub fn read_u32(mem: &SharedMemory, addr: u64) -> Option<u32> {
    let (base, len) = mem_base_len(mem); let a = addr as usize;
    if a + 4 > len { return None; }
    unsafe { Some(std::ptr::read_unaligned(base.add(a) as *const u32)) }
}
pub fn write_u64(mem: &SharedMemory, addr: u64, v: u64) -> bool { write_bytes(mem, addr, &v.to_le_bytes()) }
pub fn write_u32(mem: &SharedMemory, addr: u64, v: u32) -> bool { write_bytes(mem, addr, &v.to_le_bytes()) }
pub fn read_bytes(mem: &SharedMemory, addr: u64, n: u64) -> Vec<u8> {
    let (base, len) = mem_base_len(mem); let (a, n) = (addr as usize, n as usize);
    if a + n > len { return Vec::new(); }
    unsafe { std::slice::from_raw_parts(base.add(a), n).to_vec() }
}
pub fn write_bytes(mem: &SharedMemory, addr: u64, data: &[u8]) -> bool {
    let (base, len) = mem_base_len(mem); let a = addr as usize;
    if a + data.len() > len { return false; }
    unsafe { std::ptr::copy_nonoverlapping(data.as_ptr(), base.add(a), data.len()); }
    true
}
pub fn read_cstr(mem: &SharedMemory, addr: u64) -> String {
    let (base, len) = mem_base_len(mem);
    let mut a = addr as usize; let mut out = Vec::new();
    while a < len && out.len() < 65536 {
        let b = unsafe { *base.add(a) }; if b == 0 { break; }
        out.push(b); a += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

// ============================================================
// Метрики памяти (заглушки для Android, реальные значения — на iOS)
// ============================================================
pub fn code_embedded() -> bool {
    false
}

pub fn footprint_mb() -> u64 {
    0
}

pub fn heap_resident_mb() -> u64 {
    0
}

pub fn malloc_mb() -> u64 {
    0
}

pub fn gpu_allocated_mb() -> u64 {
    0
}

pub fn compressed_mb() -> u64 {
    0
}

// ============================================================
// JNI entry point для Android
// ============================================================
#[cfg(target_os = "android")]
#[no_mangle]
pub extern "system" fn Java_com_skipp_vesta_WasmRuntime_nativeRun(
    mut env: jni::JNIEnv,
    _class: jni::objects::JClass,
    media_path: jni::objects::JString,
) -> jni::sys::jint {
    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(log::LevelFilter::Info)
            .with_tag("Muguet"),
    );

    let path: String = match env.get_string(&media_path) {
        Ok(s) => s.into(),
        Err(e) => {
            log::error!("Muguet: не удалось прочитать media_path: {e:?}");
            return -1;
        }
    };

    log::info!("Muguet JNI: nativeRun called with MEDIA={path}");
    std::env::set_var("MUGUET_HOME", &path);

    std::thread::spawn(move || {
        // Сбрасываем прошлый прогресс
        let _ = std::fs::write(format!("{}/progress.txt", std::env::var("MUGUET_HOME").unwrap()), "PROGRESS|0|Старт...");
        log::info!("Muguet: run_muguet() начал выполнение");
        match run_muguet() {
            Ok(()) => {
                log::info!("Muguet: run_muguet() вернулся Ok");
                report_done("Движок запущен");
            }
            Err(e) => {
                let msg = format!("{e:#}");
                log::error!("Muguet: run_muguet() упал: {msg}");
                report_error(&msg);
            }
        }
    });

    0
}

// ============================================================
// Отчёт о прогрессе для Android UI (через файл)
// ============================================================
#[cfg(target_os = "android")]
pub fn report_progress(percent: u32, message: &str) {
    if let Ok(home) = std::env::var("MUGUET_HOME") {
        let path = format!("{home}/progress.txt");
        let _ = std::fs::write(&path, format!("PROGRESS|{percent}|{message}"));
    }
    log::info!("[progress] {percent}% {message}");
}

#[cfg(target_os = "android")]
pub fn report_done(message: &str) {
    if let Ok(home) = std::env::var("MUGUET_HOME") {
        let path = format!("{home}/progress.txt");
        let _ = std::fs::write(&path, format!("OK|100|{message}"));
    }
}

#[cfg(target_os = "android")]
pub fn report_error(message: &str) {
    if let Ok(home) = std::env::var("MUGUET_HOME") {
        let path = format!("{home}/progress.txt");
        let _ = std::fs::write(&path, format!("ERR|0|{message}"));
    }
}

#[cfg(not(target_os = "android"))]
pub fn report_progress(_: u32, _: &str) {}
#[cfg(not(target_os = "android"))]
pub fn report_done(_: &str) {}
#[cfg(not(target_os = "android"))]
pub fn report_error(_: &str) {}

// ============================================================
// JNI-функции для Android SurfaceView
// ============================================================
#[cfg(target_os = "android")]
pub mod android_surface {
    use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU32, Ordering};
    use std::sync::Mutex;

    /// ANativeWindow* как usize (потому что *mut T не Send/Sync)
    static NATIVE_WINDOW: Mutex<Option<usize>> = Mutex::new(None);
    pub static SURFACE_READY: AtomicBool = AtomicBool::new(false);
    pub static SURFACE_WIDTH: AtomicU32 = AtomicU32::new(0);
    pub static SURFACE_HEIGHT: AtomicU32 = AtomicU32::new(0);
    pub static SURFACE_REMOVED: AtomicBool = AtomicBool::new(false);

    pub fn set_native_window(ptr: usize) {
        *NATIVE_WINDOW.lock().unwrap() = Some(ptr);
        SURFACE_REMOVED.store(false, Ordering::SeqCst);
        SURFACE_READY.store(true, Ordering::SeqCst);
    }

    pub fn take_native_window() -> Option<usize> {
        NATIVE_WINDOW.lock().unwrap().take()
    }

    pub fn clear() {
        *NATIVE_WINDOW.lock().unwrap() = None;
        SURFACE_READY.store(false, Ordering::SeqCst);
        SURFACE_REMOVED.store(true, Ordering::SeqCst);
    }
}

#[cfg(target_os = "android")]
#[no_mangle]
pub extern "system" fn Java_com_skipp_vesta_WasmRuntime_nativeSurfaceCreated(
    mut env: jni::JNIEnv,
    _class: jni::objects::JClass,
    surface: jni::objects::JObject,
    width: jni::sys::jint,
    height: jni::sys::jint,
) {
    use std::sync::atomic::Ordering;

    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(log::LevelFilter::Info)
            .with_tag("Muguet"),
    );

    log::info!("Muguet JNI: nativeSurfaceCreated({width}x{height})");

    unsafe {
        let window = ndk_sys::ANativeWindow_fromSurface(
            env.get_native_interface(),
            surface.as_raw(),
        );
        if window.is_null() {
            log::error!("Muguet JNI: ANativeWindow_fromSurface вернул null");
            return;
        }
        android_surface::set_native_window(window as usize);
        android_surface::SURFACE_WIDTH.store(width as u32, Ordering::SeqCst);
        android_surface::SURFACE_HEIGHT.store(height as u32, Ordering::SeqCst);
        log::info!("Muguet JNI: ANativeWindow = {:p}", window);
    }
}

#[cfg(target_os = "android")]
#[no_mangle]
pub extern "system" fn Java_com_skipp_vesta_WasmRuntime_nativeSurfaceDestroyed(
    _env: jni::JNIEnv,
    _class: jni::objects::JClass,
) {
    log::info!("Muguet JNI: nativeSurfaceDestroyed");
    android_surface::clear();
}