use camunda_core::models::{RestartPlan, UpdateDownloadProgress, UpdateDownloadStage};

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
            Status::Idle => "Обновления проверяются автоматически раз в шесть часов".into(),
            Status::Checking => "Проверяем GitHub Releases…".into(),
            Status::UpToDate => "Установлена последняя версия".into(),
            Status::Downloading(version) => format!("Версия {version}: {}", stage(progress)),
            Status::Ready { version, .. } => format!("Версия {version} установлена, применится после перезапуска"),
            Status::Opened(file) => format!("Тихо поставить не вышло, открыт установщик {file}"),
            Status::Failed(message) => message.clone(),
        }
    }
}

fn stage(progress: Option<&UpdateDownloadProgress>) -> String {
    let Some(progress) = progress else { return "готовим загрузку…".into() };
    let megabytes = |bytes: i64| bytes as f64 / 1_048_576.;
    match progress.stage {
        UpdateDownloadStage::Preparing => "готовим загрузку…".into(),
        UpdateDownloadStage::Downloading if progress.total_bytes > 0 => {
            format!("скачиваем {:.1} из {:.1} МБ", megabytes(progress.downloaded_bytes), megabytes(progress.total_bytes))
        }
        UpdateDownloadStage::Downloading => format!("скачиваем {:.1} МБ", megabytes(progress.downloaded_bytes)),
        UpdateDownloadStage::Verifying => "проверяем контрольную сумму…".into(),
        UpdateDownloadStage::Installing => "устанавливаем…".into(),
    }
}
