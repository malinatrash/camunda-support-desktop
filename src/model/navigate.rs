#[derive(Clone)]
pub enum Navigate {
    Definition { id: String, all_versions: bool },
    Instance(String),
    Back,
}
