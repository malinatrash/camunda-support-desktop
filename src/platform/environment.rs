use camunda_core::models::Environment;
use gpui_kit::component::Sizable;
use gpui_kit::component::tag::Tag;
use gpui_kit::*;

pub const ALL: [Environment; 4] = [Environment::Production, Environment::Test, Environment::Development, Environment::Local];

pub fn label(environment: &Environment) -> SharedString {
    match environment {
        Environment::Production => tr!("model.environment.production"),
        Environment::Test => tr!("model.environment.test"),
        Environment::Development => tr!("model.environment.development"),
        Environment::Local => tr!("model.environment.local"),
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
