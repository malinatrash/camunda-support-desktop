package com.malinatrash.camundasupport.ui.components

import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier

@Composable
expect fun BpmnViewer(
    xml: String,
    activeActivityIds: Set<String>,
    incidentActivityIds: Set<String>,
    clickableActivityIds: Set<String> = emptySet(),
    onActivityClick: (String) -> Unit = {},
    modifier: Modifier = Modifier,
)
