package com.malinatrash.camundasupport.data

interface ExternalNavigator {
    fun open(url: String)
}
object NoOpExternalNavigator : ExternalNavigator {
    override fun open(url: String) = Unit
}
