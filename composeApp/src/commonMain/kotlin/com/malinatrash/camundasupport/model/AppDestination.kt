package com.malinatrash.camundasupport.model

enum class AppDestination(
    val title: String,
    val description: String,
    val marker: String,
    val visibleInSidebar: Boolean = true,
) {
    Overview("Дашборд", "Состояние подключения и процессов", "01"),
    Processes("Поиск заявок", "Поиск экземпляров по переменным", "02"),
    Incidents("Инциденты", "Открытые инциденты Camunda", "03"),
    Connections("Подключения", "Окружения Camunda", "04"),
    Settings("Настройки", "Параметры приложения", "05"),
    ProcessDefinition("Процесс", "Экземпляры выбранного процесса", "", false),
    ProcessInstance("Заявка", "Карточка экземпляра процесса", "", false),
}
