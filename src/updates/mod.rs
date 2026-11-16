//! Автообновление: фоновая проверка GitHub Releases, тихая установка и перезапуск по кнопке.

mod notice;
mod status;

use std::sync::{Arc, Mutex};
use std::time::Duration;

use camunda_core::models::{AppUpdate, RestartPlan, UpdateCheckResult, UpdateDownloadProgress, UpdateInstallResult};
use camunda_core::updater::{self, AppUpdateService};
use gpui_kit::*;

pub use notice::announce;
pub use status::Status;

use crate::services;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
const FIRST_CHECK_AFTER: Duration = Duration::from_secs(3);
const CHECK_EVERY: Duration = Duration::from_secs(6 * 60 * 60);
const PROGRESS_REFRESH: Duration = Duration::from_millis(150);

pub struct Updater {
    status: Status,
    progress: Arc<Mutex<Option<UpdateDownloadProgress>>>,
    announced: bool,
    work: Option<Task<()>>,
    _poll: Task<()>,
}

struct GlobalUpdater(Entity<Updater>);

impl Global for GlobalUpdater {}

impl Updater {
    pub fn init(cx: &mut App) {
        let updater = cx.new(|cx: &mut Context<Self>| Self {
            status: Status::Idle,
            progress: Arc::default(),
            announced: false,
            work: None,
            _poll: cx.spawn(async move |this, cx| {
                cx.background_executor().timer(FIRST_CHECK_AFTER).await;
                while this.update(cx, |this: &mut Self, cx| this.check(cx)).is_ok() {
                    cx.background_executor().timer(CHECK_EVERY).await;
                }
            }),
        });
        cx.set_global(GlobalUpdater(updater));
    }

    pub fn global(cx: &App) -> Entity<Self> {
        cx.global::<GlobalUpdater>().0.clone()
    }

    pub fn status(&self) -> &Status {
        &self.status
    }

    pub fn progress(&self) -> Option<UpdateDownloadProgress> {
        self.progress.lock().ok().and_then(|progress| progress.clone())
    }

    pub fn check(&mut self, cx: &mut Context<Self>) {
        if self.status.is_busy() || matches!(self.status, Status::Ready { .. }) {
            return;
        }
        self.status = Status::Checking;
        let check = services::request(cx, |_| async { Ok(AppUpdateService::new(VERSION).check_for_update(VERSION).await) });
        self.work = Some(cx.spawn(async move |this, cx| {
            let result = check.await;
            this.update(cx, |this, cx| match result {
                Ok(UpdateCheckResult::Available { update }) => this.install(update, cx),
                Ok(UpdateCheckResult::UpToDate) => this.finish(Status::UpToDate, cx),
                Ok(UpdateCheckResult::Failure { message }) | Err(message) => this.finish(Status::Failed(message), cx),
            })
            .ok();
        }));
        cx.notify();
    }

    fn install(&mut self, update: AppUpdate, cx: &mut Context<Self>) {
        let progress = self.progress.clone();
        let report = move |value: UpdateDownloadProgress| {
            if let Ok(mut slot) = progress.lock() {
                *slot = Some(value);
            }
        };
        let target = update.clone();
        let install = services::request(cx, move |_| async move { Ok(AppUpdateService::new(VERSION).download_and_install(&target, &report).await) });
        self.status = Status::Downloading(update.version);
        self.work = Some(cx.spawn(async move |this, cx| {
            let mut install = std::pin::pin!(install);
            let result = loop {
                let tick = cx.background_executor().timer(PROGRESS_REFRESH);
                match futures::future::select(install.as_mut(), tick).await {
                    futures::future::Either::Left((result, _)) => break result,
                    futures::future::Either::Right(_) => {
                        if this.update(cx, |_, cx| cx.notify()).is_err() {
                            return;
                        }
                    }
                }
            };
            this.update(cx, |this, cx| {
                let status = match result {
                    Ok(UpdateInstallResult::Installed { version, restart }) => Status::Ready { version, restart },
                    Ok(UpdateInstallResult::InstallerOpened { file_name }) => Status::Opened(file_name),
                    Ok(UpdateInstallResult::Failure { message }) | Err(message) => Status::Failed(message),
                };
                this.finish(status, cx);
            })
            .ok();
        }));
        cx.notify();
    }

    fn finish(&mut self, status: Status, cx: &mut Context<Self>) {
        self.status = status;
        self.work = None;
        cx.notify();
    }

    pub fn restart(cx: &mut App) {
        let plan: Option<RestartPlan> = match Self::global(cx).read(cx).status() {
            Status::Ready { restart, .. } => Some(restart.clone()),
            _ => None,
        };
        let Some(plan) = plan else { return };
        match updater::restart(&plan) {
            Ok(()) => cx.quit(),
            Err(message) => Self::global(cx).update(cx, |this, cx| this.finish(Status::Failed(message), cx)),
        }
    }
}
