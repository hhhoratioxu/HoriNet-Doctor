use horinet_core::{
    diagnostics,
    models::*,
    network::{connectivity, dns, monitor, proxy, traceroute},
    reports,
    storage::Store,
    validation,
};
use serde::{Deserialize, Serialize};
use std::{sync::Mutex, time::Duration};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_dialog::DialogExt;
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;
type CmdResult<T> = Result<T, String>;
fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}
fn lock<T>(m: &Mutex<T>) -> CmdResult<std::sync::MutexGuard<'_, T>> {
    m.lock()
        .map_err(|_| "Application state lock unavailable".into())
}
struct Job {
    id: String,
    token: CancellationToken,
    pause: watch::Sender<bool>,
}
pub struct AppState {
    store: Mutex<Store>,
    job: Mutex<Option<Job>>,
}
impl AppState {
    pub fn new(store: Store) -> Self {
        Self {
            store: Mutex::new(store),
            job: Mutex::new(None),
        }
    }
    pub fn cancel_all(&self) {
        if let Ok(job) = self.job.lock() {
            if let Some(job) = job.as_ref() {
                job.token.cancel();
            }
        }
    }
    fn begin(&self, id: &str) -> CmdResult<(CancellationToken, watch::Receiver<bool>)> {
        uuid::Uuid::parse_str(id).map_err(err)?;
        let mut active = lock(&self.job)?;
        if active.is_some() {
            return Err("A network job is already running; stop it before starting another".into());
        }
        let token = CancellationToken::new();
        let (tx, rx) = watch::channel(false);
        *active = Some(Job {
            id: id.into(),
            token: token.clone(),
            pause: tx,
        });
        Ok((token, rx))
    }
    fn settings(&self) -> CmdResult<Settings> {
        lock(&self.store)?.settings().map_err(err)
    }
}
#[derive(Clone, Serialize, Default)]
struct Event {
    job_id: String,
    kind: String,
    done: bool,
    completed: Option<usize>,
    total: Option<usize>,
    check: Option<Check>,
    report: Option<Report>,
    sample: Option<Sample>,
    hop: Option<Hop>,
    dns: Option<DnsResult>,
    proxy: Option<ProxyTest>,
    tcp_ms: Option<f64>,
    error: Option<String>,
}
fn emit(app: &AppHandle, event: Event) {
    let _ = app.emit("doctor-event", event);
}
fn event(id: &str, kind: &str) -> Event {
    Event {
        job_id: id.into(),
        kind: kind.into(),
        ..Default::default()
    }
}
fn finish(app: &AppHandle, id: &str, kind: &str, error: Option<String>) {
    if let Ok(mut active) = app.state::<AppState>().job.lock() {
        if active.as_ref().is_some_and(|j| j.id == id) {
            *active = None;
        }
    }
    emit(
        app,
        Event {
            done: true,
            error,
            ..event(id, kind)
        },
    );
}
struct Guard {
    app: AppHandle,
    id: String,
    kind: String,
    done: bool,
}
impl Guard {
    fn finish(mut self, error: Option<String>) {
        finish(&self.app, &self.id, &self.kind, error);
        self.done = true;
    }
}
impl Drop for Guard {
    fn drop(&mut self) {
        if !self.done {
            finish(
                &self.app,
                &self.id,
                &self.kind,
                Some("Job ended unexpectedly".into()),
            );
        }
    }
}
fn guard(app: &AppHandle, id: &str, kind: &str) -> Guard {
    Guard {
        app: app.clone(),
        id: id.into(),
        kind: kind.into(),
        done: false,
    }
}

#[tauri::command]
pub fn get_settings(state: State<AppState>) -> CmdResult<Settings> {
    state.settings()
}
#[tauri::command]
pub fn save_settings(state: State<AppState>, settings: Settings) -> CmdResult<()> {
    lock(&state.store)?.save_settings(&settings).map_err(err)
}
#[tauri::command]
pub async fn inspect_network(app: AppHandle) -> CmdResult<Snapshot> {
    let id = uuid::Uuid::new_v4().to_string();
    let (token, _) = app.state::<AppState>().begin(&id)?;
    let g = guard(&app, &id, "inspect");
    let result = diagnostics::inspect(&token).await.map_err(err);
    g.finish(None);
    result
}
#[tauri::command]
pub fn start_diagnosis(app: AppHandle, state: State<AppState>, job_id: String) -> CmdResult<()> {
    let settings = state.settings()?;
    let (token, _) = state.begin(&job_id)?;
    tauri::async_runtime::spawn(async move {
        let g = guard(&app, &job_id, "diagnose");
        let r = diagnostics::run(settings.clone(), token, |n, c| {
            emit(
                &app,
                Event {
                    completed: Some(n),
                    total: Some(diagnostics::TOTAL),
                    check: Some(c),
                    ..event(&job_id, "diagnose")
                },
            )
        })
        .await;
        match r {
            Ok(report) => {
                let saved = lock(&app.state::<AppState>().store)
                    .and_then(|s| s.save(&report, settings.retention).map_err(err));
                emit(
                    &app,
                    Event {
                        report: Some(report),
                        ..event(&job_id, "diagnose")
                    },
                );
                g.finish(saved.err());
            }
            Err(e) => g.finish(Some(e.to_string())),
        }
    });
    Ok(())
}
#[tauri::command]
pub fn start_monitor(
    app: AppHandle,
    state: State<AppState>,
    job_id: String,
    target: String,
    port: u16,
) -> CmdResult<()> {
    validation::host(&target).map_err(err)?;
    if port == 0 {
        return Err("Port must be 1..65535".into());
    }
    let s = state.settings()?;
    let (token, pause) = state.begin(&job_id)?;
    tauri::async_runtime::spawn(async move {
        let g = guard(&app, &job_id, "monitor");
        let r = monitor::monitor(
            &target,
            port,
            Duration::from_millis(s.interval_ms),
            Duration::from_millis(s.timeout_ms),
            pause,
            &token,
            |sample| {
                emit(
                    &app,
                    Event {
                        sample: Some(sample),
                        ..event(&job_id, "monitor")
                    },
                )
            },
        )
        .await;
        g.finish(r.err().map(err));
    });
    Ok(())
}
#[tauri::command]
pub fn start_trace(
    app: AppHandle,
    state: State<AppState>,
    job_id: String,
    target: String,
    ipv6: bool,
    max_hops: u8,
) -> CmdResult<()> {
    validation::host(&target).map_err(err)?;
    if !(1..=30).contains(&max_hops) {
        return Err("Hop limit must be 1..30".into());
    }
    let s = state.settings()?;
    let (token, _) = state.begin(&job_id)?;
    tauri::async_runtime::spawn(async move {
        let g = guard(&app, &job_id, "trace");
        let r = traceroute::trace(
            &target,
            ipv6,
            max_hops,
            Duration::from_millis(s.timeout_ms),
            &token,
            |hop| {
                emit(
                    &app,
                    Event {
                        hop: Some(hop),
                        ..event(&job_id, "trace")
                    },
                )
            },
        )
        .await;
        g.finish(r.err().map(err));
    });
    Ok(())
}
#[derive(Deserialize)]
pub struct DnsInput {
    domain: String,
    servers: Vec<String>,
    doh: bool,
}
#[tauri::command]
pub fn start_dns(
    app: AppHandle,
    state: State<AppState>,
    job_id: String,
    input: DnsInput,
) -> CmdResult<()> {
    dns::query_packet(&input.domain, false, 0).map_err(err)?;
    if input.servers.len() > 3 || (input.servers.is_empty() && !input.doh) {
        return Err("Choose 1..3 DNS endpoints and/or enable DoH".into());
    }
    for s in &input.servers {
        validation::endpoint(s).map_err(err)?;
    }
    let cfg = state.settings()?;
    let (token, _) = state.begin(&job_id)?;
    tauri::async_runtime::spawn(async move {
        let g = guard(&app, &job_id, "dns");
        let timeout = Duration::from_millis(cfg.timeout_ms);
        for server in &input.servers {
            for transport in ["udp", "tcp"] {
                for ipv6 in [false, true] {
                    match dns::query(&input.domain, server, transport, ipv6, timeout, &token).await
                    {
                        Ok(result) => emit(
                            &app,
                            Event {
                                dns: Some(result),
                                ..event(&job_id, "dns")
                            },
                        ),
                        Err(e) => {
                            g.finish(Some(e.to_string()));
                            return;
                        }
                    }
                }
            }
        }
        if input.doh {
            for ipv6 in [false, true] {
                match dns::query(&input.domain, &cfg.doh_url, "doh", ipv6, timeout, &token).await {
                    Ok(result) => emit(
                        &app,
                        Event {
                            dns: Some(result),
                            ..event(&job_id, "dns")
                        },
                    ),
                    Err(e) => {
                        g.finish(Some(e.to_string()));
                        return;
                    }
                }
            }
        }
        g.finish(None);
    });
    Ok(())
}
#[tauri::command]
pub fn start_proxy(
    app: AppHandle,
    state: State<AppState>,
    job_id: String,
    endpoint: String,
    protocol: String,
) -> CmdResult<()> {
    validation::endpoint(&endpoint).map_err(err)?;
    if !matches!(protocol.as_str(), "http" | "socks5") {
        return Err("Choose HTTP or SOCKS5".into());
    }
    let s = state.settings()?;
    let (token, _) = state.begin(&job_id)?;
    tauri::async_runtime::spawn(async move {
        let g = guard(&app, &job_id, "proxy");
        match proxy::test(
            &endpoint,
            &protocol,
            &s.https_url,
            Duration::from_millis(s.timeout_ms),
            &token,
        )
        .await
        {
            Ok(result) => {
                emit(
                    &app,
                    Event {
                        proxy: Some(result),
                        ..event(&job_id, "proxy")
                    },
                );
                g.finish(None);
            }
            Err(e) => g.finish(Some(e.to_string())),
        }
    });
    Ok(())
}
#[tauri::command]
pub fn test_tcp(
    app: AppHandle,
    state: State<AppState>,
    job_id: String,
    endpoint: String,
) -> CmdResult<()> {
    let address = validation::endpoint(&endpoint).map_err(err)?;
    let s = state.settings()?;
    let (token, _) = state.begin(&job_id)?;
    tauri::async_runtime::spawn(async move {
        let g = guard(&app, &job_id, "tcp");
        match connectivity::tcp(address, Duration::from_millis(s.timeout_ms), &token).await {
            Ok(ms) => {
                emit(
                    &app,
                    Event {
                        tcp_ms: Some(ms),
                        ..event(&job_id, "tcp")
                    },
                );
                g.finish(None);
            }
            Err(e) => g.finish(Some(e.to_string())),
        }
    });
    Ok(())
}
#[tauri::command]
pub fn cancel_job(state: State<AppState>, job_id: String) -> CmdResult<()> {
    let job = lock(&state.job)?;
    if let Some(j) = job.as_ref().filter(|j| j.id == job_id) {
        j.token.cancel();
    }
    Ok(())
}
#[tauri::command]
pub fn pause_monitor(state: State<AppState>, job_id: String, paused: bool) -> CmdResult<()> {
    let job = lock(&state.job)?;
    let j = job
        .as_ref()
        .filter(|j| j.id == job_id)
        .ok_or("Job not running")?;
    j.pause.send(paused).map_err(err)
}
#[tauri::command]
pub async fn list_history(app: AppHandle) -> CmdResult<Vec<Report>> {
    tauri::async_runtime::spawn_blocking(move || {
        lock(&app.state::<AppState>().store)?.list().map_err(err)
    })
    .await
    .map_err(err)?
}
#[tauri::command]
pub fn get_report(state: State<AppState>, id: String) -> CmdResult<Report> {
    lock(&state.store)?.get(&id).map_err(err)
}
#[tauri::command]
pub fn delete_history(state: State<AppState>, id: String) -> CmdResult<()> {
    lock(&state.store)?.delete(&id).map_err(err)
}
#[tauri::command]
pub fn clear_history(state: State<AppState>) -> CmdResult<()> {
    lock(&state.store)?.clear().map_err(err)
}
#[tauri::command]
pub fn copy_summary(state: State<AppState>, id: String, detailed: bool) -> CmdResult<String> {
    Ok(reports::summary(
        &lock(&state.store)?.get(&id).map_err(err)?,
        detailed,
    ))
}
#[tauri::command]
pub async fn export_report(
    app: AppHandle,
    id: String,
    format: String,
    detailed: bool,
) -> CmdResult<bool> {
    if !matches!(format.as_str(), "json" | "md") {
        return Err("Choose JSON or Markdown".into());
    }
    let report = lock(&app.state::<AppState>().store)?
        .get(&id)
        .map_err(err)?;
    let text = if format == "json" {
        reports::json(&report, detailed).map_err(err)?
    } else {
        reports::markdown(&report, detailed)
    };
    save_dialog(
        app,
        format!("HoriNet-{}.{}", report.id, format),
        format,
        text,
    )
    .await
}
#[tauri::command]
pub async fn export_trace(app: AppHandle, hops: Vec<Hop>, detailed: bool) -> CmdResult<bool> {
    if hops.len() > 30 {
        return Err("Too many hops".into());
    }
    let mut out=String::from("HoriNet Doctor — traceroute\nUnknown hops do not establish line failure.\nTTL\tAddress\tRTT ms\tStatus\n");
    for h in hops {
        let address = if detailed {
            h.reply
                .address
                .as_deref()
                .and_then(|s| s.parse::<std::net::IpAddr>().ok())
                .map_or("unknown".into(), |a| a.to_string())
        } else {
            "[redacted]".into()
        };
        out.push_str(&format!(
            "{}\t{}\t{:?}\t{}\n",
            h.ttl, address, h.reply.rtt_ms, h.reply.status_code
        ));
    }
    save_dialog(app, "HoriNet-traceroute.txt".into(), "txt".into(), out).await
}
async fn save_dialog(
    app: AppHandle,
    name: String,
    extension: String,
    text: String,
) -> CmdResult<bool> {
    tauri::async_runtime::spawn_blocking(move || {
        let path = app
            .dialog()
            .file()
            .set_file_name(name)
            .add_filter("Report", &[&extension])
            .blocking_save_file();
        if let Some(path) = path {
            let path = path.into_path().map_err(err)?;
            std::fs::write(path, text).map_err(err)?;
            Ok(true)
        } else {
            Ok(false)
        }
    })
    .await
    .map_err(err)?
}
#[tauri::command]
pub fn open_repository(app: AppHandle) -> CmdResult<()> {
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_url(
            "https://github.com/hhhoratioxu/HoriNet-Doctor",
            None::<&str>,
        )
        .map_err(err)
}
