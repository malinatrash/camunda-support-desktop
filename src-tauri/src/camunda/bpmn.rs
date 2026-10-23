use std::collections::HashMap;

use roxmltree::{Document, Node};

use crate::models::{BpmnDiagram, BpmnEdge, BpmnNode, BpmnPoint};

// Разбирает BPMN XML в каталог узлов и переходов по BPMNShape/BPMNEdge; roxmltree не раскрывает внешние сущности.
pub fn parse_bpmn(xml: &str) -> Result<BpmnDiagram, String> {
    if xml.trim().is_empty() {
        return Ok(BpmnDiagram::default());
    }
    let document = Document::parse(xml).map_err(|error| format!("Не удалось разобрать BPMN XML: {error}"))?;
    let elements: Vec<Node> = document.descendants().filter(Node::is_element).collect();
    let semantic_by_id: HashMap<&str, Node> = elements
        .iter()
        .filter_map(|element| non_blank(element.attribute("id")).map(|id| (id, *element)))
        .collect();

    let nodes = elements
        .iter()
        .filter(|element| element.tag_name().name() == "BPMNShape")
        .filter_map(|shape| {
            let element_id = non_blank(shape.attribute("bpmnElement"))?;
            let bounds = child_element(shape, "Bounds")?;
            let semantic = semantic_by_id.get(element_id);
            Some(BpmnNode {
                id: element_id.to_string(),
                name: semantic.and_then(|semantic| non_blank(semantic.attribute("name"))).map(str::to_string),
                node_type: semantic.map(|semantic| semantic.tag_name().name().to_string()).unwrap_or_else(|| "unknown".to_string()),
                x: float_attribute(&bounds, "x"),
                y: float_attribute(&bounds, "y"),
                width: float_attribute(&bounds, "width"),
                height: float_attribute(&bounds, "height"),
                topic: semantic.and_then(|semantic| attribute_by_local_name(semantic, "topic")).map(str::to_string),
            })
        })
        .collect();

    let edges = elements
        .iter()
        .filter(|element| element.tag_name().name() == "BPMNEdge")
        .filter_map(|edge| {
            let element_id = non_blank(edge.attribute("bpmnElement"))?;
            let semantic = semantic_by_id.get(element_id);
            Some(BpmnEdge {
                id: element_id.to_string(),
                source_ref: semantic.and_then(|semantic| non_blank(semantic.attribute("sourceRef"))).map(str::to_string),
                target_ref: semantic.and_then(|semantic| non_blank(semantic.attribute("targetRef"))).map(str::to_string),
                waypoints: edge
                    .children()
                    .filter(|child| child.is_element() && child.tag_name().name() == "waypoint")
                    .map(|waypoint| BpmnPoint { x: float_attribute(&waypoint, "x"), y: float_attribute(&waypoint, "y") })
                    .collect(),
            })
        })
        .collect();

    Ok(BpmnDiagram { nodes, edges })
}

fn non_blank(value: Option<&str>) -> Option<&str> {
    value.filter(|value| !value.trim().is_empty())
}

fn child_element<'a, 'input>(parent: &Node<'a, 'input>, local_name: &str) -> Option<Node<'a, 'input>> {
    parent.children().find(|child| child.is_element() && child.tag_name().name() == local_name)
}

fn attribute_by_local_name<'a>(node: &Node<'a, '_>, local_name: &str) -> Option<&'a str> {
    non_blank(node.attributes().find(|attribute| attribute.name() == local_name).map(|attribute| attribute.value()))
}

fn float_attribute(node: &Node, name: &str) -> f32 {
    node.attribute(name).and_then(|value| value.parse::<f32>().ok()).unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const DRZI: &str = include_str!("../../tests/fixtures/drzi.bpmn");

    #[test]
    fn parses_diagram_coordinates_and_teleport_targets() {
        let diagram = parse_bpmn(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL"
    xmlns:bpmndi="http://www.omg.org/spec/BPMN/20100524/DI"
    xmlns:dc="http://www.omg.org/spec/DD/20100524/DC"
    xmlns:di="http://www.omg.org/spec/DD/20100524/DI"
    xmlns:camunda="http://camunda.org/schema/1.0/bpmn">
  <process id="Process_1">
    <startEvent id="Start_1" name="Старт" />
    <serviceTask id="Task_1" name="Проверить клиента" camunda:type="external" camunda:topic="customer-check" />
    <sequenceFlow id="Flow_1" sourceRef="Start_1" targetRef="Task_1" />
  </process>
  <bpmndi:BPMNDiagram id="Diagram_1">
    <bpmndi:BPMNPlane id="Plane_1" bpmnElement="Process_1">
      <bpmndi:BPMNShape id="Start_shape" bpmnElement="Start_1">
        <dc:Bounds x="100" y="120" width="36" height="36" />
      </bpmndi:BPMNShape>
      <bpmndi:BPMNShape id="Task_shape" bpmnElement="Task_1">
        <dc:Bounds x="200" y="90" width="110" height="80" />
      </bpmndi:BPMNShape>
      <bpmndi:BPMNEdge id="Flow_edge" bpmnElement="Flow_1">
        <di:waypoint x="136" y="138" />
        <di:waypoint x="200" y="130" />
      </bpmndi:BPMNEdge>
    </bpmndi:BPMNPlane>
  </bpmndi:BPMNDiagram>
</definitions>"#,
        )
        .expect("диаграмма");

        assert_eq!(diagram.nodes.iter().map(|node| node.id.as_str()).collect::<Vec<_>>(), vec!["Start_1", "Task_1"]);
        let task = diagram.nodes.last().expect("узел");
        assert_eq!(task.node_type, "serviceTask");
        assert_eq!(task.topic.as_deref(), Some("customer-check"));
        assert_eq!(task.name.as_deref(), Some("Проверить клиента"));
        assert_eq!((task.x, task.y, task.width, task.height), (200.0, 90.0, 110.0, 80.0));
        let edge = &diagram.edges[0];
        assert_eq!(diagram.edges.len(), 1);
        assert_eq!(edge.waypoints.len(), 2);
        assert_eq!(edge.source_ref.as_deref(), Some("Start_1"));
        assert_eq!(edge.target_ref.as_deref(), Some("Task_1"));
        assert_eq!(diagram.teleport_targets().map(|node| node.id.as_str()).collect::<Vec<_>>(), vec!["Task_1"]);
    }

    #[test]
    fn parses_nested_subprocess_diagrams_from_fixture() {
        let diagram = parse_bpmn(DRZI).expect("диаграмма drzi");
        assert_eq!(diagram.nodes.len(), 10);
        assert_eq!(diagram.edges.len(), 7);
        let subprocess = diagram.node("Activity_1gooqz2").expect("подпроцесс");
        assert_eq!(subprocess.node_type, "subProcess");
        assert_eq!(subprocess.name.as_deref(), Some("Подготовить изменения"));
        assert_eq!(diagram.node("Activity_approve").map(|node| node.node_type.as_str()), Some("userTask"));
        let targets: Vec<&str> = diagram.teleport_targets().map(|node| node.id.as_str()).collect();
        assert_eq!(targets, vec!["Activity_1gooqz2", "Activity_1xqooxf", "Activity_0soegtg", "Activity_approve"]);
        assert!(diagram.edges.iter().all(|edge| edge.source_ref.is_some() && edge.target_ref.is_some()));
    }

    #[test]
    fn empty_xml_gives_empty_diagram_and_broken_xml_fails() {
        assert_eq!(parse_bpmn(""), Ok(BpmnDiagram::default()));
        assert_eq!(parse_bpmn("   \n"), Ok(BpmnDiagram::default()));
        assert!(parse_bpmn("<definitions><process></definitions>").is_err());
    }

    #[test]
    fn shape_without_semantic_element_is_unknown() {
        let diagram = parse_bpmn(
            r#"<definitions xmlns:bpmndi="http://www.omg.org/spec/BPMN/20100524/DI" xmlns:dc="http://www.omg.org/spec/DD/20100524/DC">
  <bpmndi:BPMNDiagram><bpmndi:BPMNPlane>
    <bpmndi:BPMNShape id="s" bpmnElement="Ghost"><dc:Bounds x="1.5" y="2" width="oops" height="4" /></bpmndi:BPMNShape>
    <bpmndi:BPMNShape id="s2" bpmnElement="NoBounds" />
  </bpmndi:BPMNPlane></bpmndi:BPMNDiagram>
</definitions>"#,
        )
        .expect("диаграмма");
        assert_eq!(diagram.nodes.len(), 1);
        assert_eq!(diagram.nodes[0].node_type, "unknown");
        assert_eq!((diagram.nodes[0].x, diagram.nodes[0].width), (1.5, 0.0));
    }
}
