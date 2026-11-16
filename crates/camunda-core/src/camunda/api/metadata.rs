use serde_json::Value;

use super::json::string;
use crate::models::*;

pub(super) fn definition_metadata(value: &Value) -> Vec<ProcessMetadataField> {
    vec![
        metadata_field("ID определения", value, "id"),
        metadata_field("Ключ", value, "key"),
        metadata_field("Название", value, "name"),
        metadata_field("Описание", value, "description"),
        metadata_field("Версия", value, "version"),
        metadata_field("Метка версии", value, "versionTag"),
        metadata_field("Категория", value, "category"),
        metadata_field("Ресурс BPMN", value, "resource"),
        metadata_field("Ресурс диаграммы", value, "diagram"),
        metadata_field("ID развёртывания", value, "deploymentId"),
        metadata_field("ID тенанта", value, "tenantId"),
        metadata_field("Срок хранения истории", value, "historyTimeToLive"),
        metadata_field("Доступен для запуска в списке задач", value, "startableInTasklist"),
        metadata_field("Приостановлен", value, "suspended"),
    ]
}

pub(super) fn instance_metadata(value: &Value) -> Vec<ProcessMetadataField> {
    vec![
        metadata_field("ID экземпляра", value, "id"),
        metadata_field("ID определения", value, "definitionId"),
        metadata_field("Ключ определения", value, "definitionKey"),
        metadata_field("Бизнес-ключ", value, "businessKey"),
        metadata_field("ID экземпляра кейса", value, "caseInstanceId"),
        metadata_field("ID тенанта", value, "tenantId"),
        metadata_field("Завершён", value, "ended"),
        metadata_field("Приостановлен", value, "suspended"),
    ]
}

fn metadata_field(label: &str, value: &Value, key: &str) -> ProcessMetadataField {
    ProcessMetadataField { label: label.to_string(), value: string(value, key).unwrap_or_else(|| "—".to_string()) }
}
