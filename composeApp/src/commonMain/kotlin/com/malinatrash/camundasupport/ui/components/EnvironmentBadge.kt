package com.malinatrash.camundasupport.ui.components

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.malinatrash.camundasupport.model.Environment
import com.malinatrash.camundasupport.ui.theme.Danger
import com.malinatrash.camundasupport.ui.theme.Development
import com.malinatrash.camundasupport.ui.theme.TextSecondary
import com.malinatrash.camundasupport.ui.theme.Warning

@Composable
fun EnvironmentBadge(
    environment: Environment,
    modifier: Modifier = Modifier,
) {
    val color = environmentColor(environment)
    Box(
        modifier = modifier
            .background(color.copy(alpha = 0.14f), RoundedCornerShape(6.dp))
            .padding(horizontal = 8.dp, vertical = 4.dp),
    ) {
        Text(
            text = environment.shortLabel,
            color = color,
            fontSize = 10.sp,
            fontWeight = FontWeight.Bold,
            letterSpacing = 0.7.sp,
        )
    }
}
@Composable
fun environmentColor(environment: Environment): Color = when (environment) {
    Environment.Production -> Danger
    Environment.Test -> Warning
    Environment.Development -> Development
    Environment.Local -> TextSecondary
}
