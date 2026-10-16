//! Десктоп-заглушка. Запускает движок и парковку.
//! Настоящее окно (winit + wgpu) добавим позже.

use crate::{config, run_engine, Runtime};
use anyhow::Result;
use std::sync::Arc;

pub fn run(rt: Option<Arc<Runtime>>, _startup: config::Startup, args: Vec<String>) -> Result<()> {
    let rt = rt.expect("runtime");
    println!("[desktop] starting engine (graphics disabled stub)");
    run_engine(&rt, args)?;
    println!("[desktop] engine returned, parking thread");
    loop { std::thread::park(); }
}