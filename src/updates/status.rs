use camunda_core::models::{RestartPlan, UpdateDownloadProgress, UpdateDownloadStage};
use rust_i18n::t;

pub enum Status {
    Idle,
    Checking,
    UpToDate,
    Downloading(String),
    Ready { version: String, restart: RestartPlan },
    Opened(String),
    Failed(String),
}

impl Status {
    pub fn is_busy(&self) -> bool {
        matches!(self, Status::Checking | Status::Downloading(_))
    }

    pub fn describe(&self, progress: Option<&UpdateDownloadProgress>) -> String {
        match self {
            Status::Idle => t!("updates.status.idle").into_owned(),
            Status::Checking => t!("updates.status.checking").into_owned(),
            Status::UpToDate => t!("updates.status.up_to_date").into_owned(),
            Status::Downloading(version) => t!("updates.status.downloading", version = version, stage = stage(progress)).into_owned(),
            Status::Ready { version, .. } => t!("updates.status.ready", version = version).into_owned(),
            Status::Opened(file) => t!("updates.status.opened", file = file).into_owned(),
            Status::Failed(message) => message.clone(),
        }
    }
}

fn stage(progress: Option<&UpdateDownloadProgress>) -> String {
    let Some(progress) = progress else { return t!("updates.stage.preparing").into_owned() };
    let megabytes = |bytes: i64| format!("{:.1}", bytes as f64 / 1_048_576.);
    match progress.stage {
        UpdateDownloadStage::Preparing => t!("updates.stage.preparing").into_owned(),
        UpdateDownloadStage::Downloading if progress.total_bytes > 0 => {
            t!("updates.stage.downloading_of", downloaded = megabytes(progress.downloaded_bytes), total = megabytes(progress.total_bytes))
                .into_owned()
        }
        UpdateDownloadStage::Downloading => t!("updates.stage.downloading", downloaded = megabytes(progress.downloaded_bytes)).into_owned(),
        UpdateDownloadStage::Verifying => t!("updates.stage.verifying").into_owned(),
        UpdateDownloadStage::Installing => t!("updates.stage.installing").into_owned(),
    }
}
