use camunda_core::models::BpmnNode;

// Пулы, дорожки и развёрнутые подпроцессы рисуются под остальными узлами и не выбираются кликом.
pub fn is_container(node: &BpmnNode) -> bool {
    match node.node_type.as_str() {
        "participant" | "lane" => true,
        "subProcess" | "transaction" | "adHocSubProcess" => node.expanded,
        _ => false,
    }
}

pub fn is_event(node: &BpmnNode) -> bool {
    node.node_type.ends_with("Event")
}

pub fn is_gateway(node: &BpmnNode) -> bool {
    node.node_type.ends_with("Gateway")
}
