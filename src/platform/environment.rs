use camunda_core::models::Environment;
use gpui_kit::component::Sizable;
use gpui_kit::component::tag::Tag;
use gpui_kit::*;

pub const ALL: [Environment; 4] = [Environment::Production, Environment::Test, Environment::Development, Environment::Local];

pub fn label(environment: &Environment) -> &'static str {
    match environment {
        Environment::Production => "Прод",
        Environment::Test => "Тест",
        Environment::Development => "Разработка",
        Environment::Local => "Локально",
    }
}

pub fn tag(environment: &Environment) -> Tag {
    let tag = match environment {
        Environment::Production => Tag::danger(),
        Environment::Test => Tag::warning(),
        Environment::Development | Environment::Local => Tag::secondary(),
    };
    tag.small().outline().child(label(environment))
}
