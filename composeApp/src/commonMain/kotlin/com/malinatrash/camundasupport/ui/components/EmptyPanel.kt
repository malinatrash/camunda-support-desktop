package com.malinatrash.camundasupport.ui.components

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.malinatrash.camundasupport.ui.theme.Border
import com.malinatrash.camundasupport.ui.theme.Surface
import com.malinatrash.camundasupport.ui.theme.TextSecondary

@Composable
fun EmptyPanel(
    title: String,
    description: String,
    modifier: Modifier = Modifier,
    actionLabel: String? = null,
    onAction: (() -> Unit)? = null,
) {
    Card(
        modifier = modifier.fillMaxWidth(),
        colors = CardDefaults.cardColors(containerColor = Surface),
        border = BorderStroke(1.dp, Border),
        shape = RoundedCornerShape(10.dp),
    ) {
        Column(
            modifier = Modifier.fillMaxWidth().padding(22.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.spacedBy(10.dp),
        ) {
            Text(title, fontWeight = FontWeight.SemiBold)
            Text(description, color = TextSecondary)
            if (actionLabel != null && onAction != null) {
                Button(onClick = onAction) {
                    Text(actionLabel)
                }
            }
        }
    }
}
