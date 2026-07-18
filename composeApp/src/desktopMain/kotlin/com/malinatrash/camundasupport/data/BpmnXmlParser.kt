package com.malinatrash.camundasupport.data

import java.io.StringReader
import javax.xml.parsers.DocumentBuilderFactory
import com.malinatrash.camundasupport.model.BpmnDiagram
import com.malinatrash.camundasupport.model.BpmnEdge
import com.malinatrash.camundasupport.model.BpmnNode
import com.malinatrash.camundasupport.model.BpmnPoint
import org.w3c.dom.Element
import org.xml.sax.InputSource

class BpmnXmlParser {
    fun parse(xml: String): BpmnDiagram {
        if (xml.isBlank()) return BpmnDiagram(emptyList(), emptyList())
        val factory = DocumentBuilderFactory.newInstance().apply {
            isNamespaceAware = true
            isXIncludeAware = false
            isExpandEntityReferences = false
            runCatching { setFeature("http://apache.org/xml/features/disallow-doctype-decl", true) }
            runCatching { setFeature("http://xml.org/sax/features/external-general-entities", false) }
            runCatching { setFeature("http://xml.org/sax/features/external-parameter-entities", false) }
        }
        val document = factory.newDocumentBuilder().parse(InputSource(StringReader(xml)))
        val allElements = document.getElementsByTagName("*")
            .asElementSequence()
            .toList()
        val semanticById = allElements
            .mapNotNull { element -> element.getAttribute("id").takeIf(String::isNotBlank)?.let { it to element } }
            .toMap()

        val nodes = allElements
            .filter { it.localTag == "BPMNShape" }
            .mapNotNull { shape ->
                val elementId = shape.getAttribute("bpmnElement").takeIf(String::isNotBlank) ?: return@mapNotNull null
                val bounds = shape.childElements().firstOrNull { it.localTag == "Bounds" } ?: return@mapNotNull null
                val semantic = semanticById[elementId]
                BpmnNode(
                    id = elementId,
                    name = semantic?.getAttribute("name")?.takeIf(String::isNotBlank),
                    type = semantic?.localTag ?: "unknown",
                    x = bounds.floatAttribute("x"),
                    y = bounds.floatAttribute("y"),
                    width = bounds.floatAttribute("width"),
                    height = bounds.floatAttribute("height"),
                    topic = semantic?.attributeByLocalName("topic"),
                )
            }

        val edges = allElements
            .filter { it.localTag == "BPMNEdge" }
            .mapNotNull { edge ->
                val elementId = edge.getAttribute("bpmnElement").takeIf(String::isNotBlank) ?: return@mapNotNull null
                val semantic = semanticById[elementId]
                BpmnEdge(
                    id = elementId,
                    sourceRef = semantic?.getAttribute("sourceRef")?.takeIf(String::isNotBlank),
                    targetRef = semantic?.getAttribute("targetRef")?.takeIf(String::isNotBlank),
                    waypoints = edge.childElements()
                        .filter { it.localTag == "waypoint" }
                        .map { BpmnPoint(it.floatAttribute("x"), it.floatAttribute("y")) }
                        .toList(),
                )
            }

        return BpmnDiagram(nodes = nodes, edges = edges)
    }
}

private val Element.localTag: String
    get() = localName ?: tagName.substringAfter(':')

private fun Element.attributeByLocalName(name: String): String? = (0 until attributes.length)
    .asSequence()
    .map(attributes::item)
    .firstOrNull { attribute -> (attribute.localName ?: attribute.nodeName.substringAfter(':')) == name }
    ?.nodeValue
    ?.takeIf(String::isNotBlank)

private fun Element.floatAttribute(name: String): Float = getAttribute(name).toFloatOrNull() ?: 0f

private fun Element.childElements(): Sequence<Element> = sequence {
    for (index in 0 until childNodes.length) {
        val node = childNodes.item(index)
        if (node is Element) yield(node)
    }
}

private fun org.w3c.dom.NodeList.asElementSequence(): Sequence<Element> = sequence {
    for (index in 0 until length) {
        val node = item(index)
        if (node is Element) yield(node)
    }
}
