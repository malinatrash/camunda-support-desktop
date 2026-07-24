package com.malinatrash.camundasupport.ui.components

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.malinatrash.camundasupport.state.OpenProcessTab
import com.malinatrash.camundasupport.ui.theme.AppBackground
import com.malinatrash.camundasupport.ui.theme.Border
import com.malinatrash.camundasupport.ui.theme.Primary
import com.malinatrash.camundasupport.ui.theme.PrimaryMuted
import com.malinatrash.camundasupport.ui.theme.Surface
import com.malinatrash.camundasupport.ui.theme.TextPrimary
import com.malinatrash.camundasupport.ui.theme.TextSecondary

@Composable
fun OpenProcessTabs(
    tabs: List<OpenProcessTab>,
    activeTabId: String?,
    onSelect: (String) -> Unit,
    onClose: (String) -> Unit,
) {
    Row(
        modifier = Modifier.fillMaxWidth().background(AppBackground).padding(horizontal = 12.dp, vertical = 7.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        Text("ОТКРЫТЫЕ ЗАЯВКИ", color = TextSecondary, fontSize = 9.sp, fontWeight = FontWeight.Bold)
        Spacer(Modifier.width(10.dp))
        LazyRow(
            modifier = Modifier.weight(1f),
            horizontalArrangement = Arrangement.spacedBy(7.dp),
        ) {
            items(tabs, key = OpenProcessTab::id) { tab ->
                val active = tab.id == activeTabId
                Card(
                    modifier = Modifier.width(220.dp).clickable { onSelect(tab.id) },
                    colors = CardDefaults.cardColors(containerColor = if (active) PrimaryMuted else Surface),
                    border = BorderStroke(if (active) 2.dp else 1.dp, if (active) Primary else Border),
                    shape = RoundedCornerShape(9.dp),
                ) {
                    Row(
                        Modifier.fillMaxWidth().padding(horizontal = 10.dp, vertical = 7.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Column(Modifier.weight(1f)) {
                            Text(
                                tab.businessKey ?: tab.processInstanceId,
                                color = if (active) Primary else TextPrimary,
                                fontWeight = FontWeight.SemiBold,
                                fontFamily = if (tab.businessKey == null) FontFamily.Monospace else FontFamily.Default,
                                fontSize = 11.sp,
                                maxLines = 1,
                                overflow = TextOverflow.Ellipsis,
                            )
                            Text(
                                listOfNotNull(tab.connectionName, tab.processDefinitionKey).joinToString(" · "),
                                color = TextSecondary,
                                fontSize = 8.sp,
                                maxLines = 1,
                                overflow = TextOverflow.Ellipsis,
                            )
                        }
                        Text(
                            "×",
                            color = TextSecondary,
                            fontSize = 16.sp,
                            modifier = Modifier.clickable { onClose(tab.id) }.padding(horizontal = 4.dp),
                        )
                    }
                }
            }
        }
    }
}
