package com.malinatrash.camundasupport.ui.components

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.height
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.malinatrash.camundasupport.ui.theme.TextSecondary

@Composable
fun SectionTitle(
    title: String,
    description: String,
    modifier: Modifier = Modifier,
) {
    Column(modifier) {
        Text(
            text = title,
            fontSize = 22.sp,
            fontWeight = FontWeight.SemiBold,
            letterSpacing = (-0.5).sp,
        )
        Spacer(Modifier.height(3.dp))
        Text(
            text = description,
            color = TextSecondary,
            fontSize = 12.sp,
        )
    }
}
