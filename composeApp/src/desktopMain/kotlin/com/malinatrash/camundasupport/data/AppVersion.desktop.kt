package com.malinatrash.camundasupport.data

actual val APP_VERSION: String
    get() = System.getProperty("camunda.support.version")
        ?.takeIf(String::isNotBlank)
        ?: "0.0.0-dev"
