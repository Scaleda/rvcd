use crate::verilog::{VerilogGotoSource, VerilogSource};
use crate::wave::Wave;
use egui_toast::Toast;
use rfd::FileHandle;
use std::fmt::{Debug, Formatter};
use std::sync::{mpsc, Arc};

// #[derive(Debug)]
pub enum RvcdMsg {
    FileOpen(FileHandle),
    FileLoadStart(String),
    FileLoadCancel,
    FileDrag(FileHandle),
    FileOpenData(Arc<[u8]>),
    LoadingProgress(f32, usize),
    ParsingProgress(f32, u64),
    FileOpenFailed(String),
    Reload,
    UpdateWave(Wave),
    Notification(Toast),
    ServiceDataReady(Vec<u8>),
    StopService,
    UpdateSourceDir(String),
    UpdateSource(String),
    UpdateSources(Vec<VerilogSource>),
    GotNoSource,
    SetAlternativeGotoSources(Vec<VerilogGotoSource>),
    CallGotoSources(VerilogGotoSource),
    SetGotoSignals(Vec<u64>)
}

impl Debug for RvcdMsg {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self {
            RvcdMsg::Notification(_toast) => write!(f, "RvcdMsg: Toast[...]"),
            RvcdMsg::FileOpen(file) => write!(f, "RvcdMsg: FileOpen({file:?})"),
            RvcdMsg::FileOpenFailed(path) => write!(f, "RvcdMsg: FileOpenFailed {path}"),
            RvcdMsg::Reload => write!(f, "RvcdMsg: Reload"),
            RvcdMsg::UpdateWave(_) => write!(f, "RvcdMsg: UpdateWave"),
            RvcdMsg::FileOpenData(v) => write!(f, "RvcdMsg: FileOpenData({} bytes)", v.len()),
            RvcdMsg::FileDrag(_) => write!(f, "RvcdMsg: FileDrag"),
            RvcdMsg::LoadingProgress(p, sz) => {
                write!(f, "RvcdMsg: LoadingProgress({}%, {} bytes)", p * 100.0, sz)
            }
            RvcdMsg::ParsingProgress(p, pos) => {
                write!(f, "RvcdMsg: ParsingProgress({}%, #{})", p * 100.0, pos)
            }
            RvcdMsg::FileLoadStart(filepath) => write!(f, "RvcdMsg: FileLoadStart({filepath})"),
            RvcdMsg::FileLoadCancel => write!(f, "RvcdMsg: FileLoadCancel"),
            RvcdMsg::ServiceDataReady(v) => {
                write!(f, "RcdMsg: ServiceDataReady ({} bytes)", v.len())
            }
            RvcdMsg::StopService => write!(f, "RvcdMsg: StopService"),
            RvcdMsg::UpdateSources(s) => write!(f, "RvcdMsg: UpdateSources({})", s.len()),
            RvcdMsg::UpdateSourceDir(path) => write!(f, "RvcdMsg: UpdateSourceDir({})", path),
            RvcdMsg::CallGotoSources(g) => write!(f, "RvcdMg: GotoSource({:?})", g),
            RvcdMsg::SetAlternativeGotoSources(g) => {
                write!(f, "RvcdMg: SetAlternativeGotoSources({:?})", g)
            }
            RvcdMsg::GotNoSource => write!(f, "RvcdMg: GotNoSource"),
            RvcdMsg::SetGotoSignals(v) => write!(f, "RvcdMg: SetGotoSignals({})", v.len()),
            RvcdMsg::UpdateSource(path) => write!(f, "RvcdMg: UpdateSource({})", path)
        }
    }
}

/// We must assert all data in [RvcdMsg] are safe to send
unsafe impl Send for RvcdMsg {}

/// [RvcdMsg] tx-rx pair
#[derive(Debug)]
pub struct RvcdChannel {
    pub tx: mpsc::Sender<RvcdMsg>,
    pub rx: mpsc::Receiver<RvcdMsg>,
}
