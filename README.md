# Camunda Support Desktop

Кроссплатформенное desktop-приложение для специалистов поддержки Bereke, которые расследуют и восстанавливают процессы Camunda 7 на macOS и Windows.

Приложение работает полностью на клиенте: напрямую обращается к настроенным Camunda REST endpoints и не требует backend-сервиса.

## Что уже работает

- Kotlin Multiplatform и Compose Multiplatform Desktop;
- светлый полностью русский интерфейс;
- локально сохранённые подключения Camunda;
- строгая проверка `/engine-rest` и обязательный ping `/version`;
- автоматический Cockpit URL из домена REST;
- dashboard последних process definitions;
- текущая статистика instances и incidents;
- статистика за сегодня, вчера и произвольный период через History API;
- точное количество полностью завершивших процесс заявок за выбранный период;
- диаграмма динамики стартов, полных завершений и инцидентов по часам или дням;
- среднее время прохождения, доля завершения, типы инцидентов и рейтинг критичных процессов;
- топ проблемных BPMN-кубиков по количеству инцидентов;
- hover-карточки процессов на дашборде с версией, deployment, tenant, TTL и статистикой;
- внутренний экран открытых и исторических инцидентов с переходом прямо в заявку;
- фильтрация инцидентов по дням, произвольному периоду и версиям процесса;
- расширенные сведения о процессе: version tag, TTL истории, BPMN resource, deployment и доступность в Tasklist;
- сортировка по времени deployment, фильтр, список и карточечное превью;
- поиск runtime instances по variable key/value;
- сохранение variable keys отдельно для каждого connection;
- внутренний экран process definition со списком запущенных заявок и метаданными;
- внутренняя карточка заявки с вкладками: BPMN, активные элементы, переменные, инциденты, задания движка и внешние задачи;
- изменение значений runtime-переменных с сохранением типа, строгой проверкой и отдельным подтверждением для продакшена;
- официальный `bpmn-js` внутри встроенного JavaFX WebView вместо самописной отрисовки BPMN;
- офлайн-ресурсы `bpmn-js`, без загрузки Cockpit и его авторизации;
- поиск целевого кубика телепорта по названию, BPMN ID, типу и `camunda:topic`;
- History API на экране процесса для заявок за сегодня, вчера или выбранный период;
- телепорт с выбором точного `activityInstanceId` и целевого BPMN-элемента;
- retries заданий, разблокировка внешней задачи и приостановка/активация заявки;
- Cockpit открывается только отдельной явной кнопкой;
- локальные contract-тесты Camunda REST.

## Запуск

Требования:

- JDK 17 или новее; рекомендуется JDK 21;
- macOS 13+ или Windows 10+.

```bash
./gradlew :composeApp:run
```

Нативные пакеты:

```bash
./gradlew :composeApp:packageDmg
./gradlew :composeApp:packageMsi
./gradlew :composeApp:packageExe
```

Каждый нативный пакет собирается на своей операционной системе.

### Сборка для Windows

Для локальной сборки нужны:

- Windows 10/11 x64;
- JDK 21 с настроенными `JAVA_HOME` и `PATH`;
- WiX Toolset 3.x (`candle.exe` и `light.exe` должны быть доступны через `PATH`).

Запуск из PowerShell в корне проекта:

```powershell
.\scripts\build-windows.ps1
```

Готовые файлы появятся здесь:

- `composeApp\build\compose\binaries\main\msi\Camunda Support-1.0.0.msi`;
- `composeApp\build\compose\binaries\main\exe\Camunda Support-1.0.0.exe`.

Для удалённой сборки предусмотрен GitHub Actions workflow `Windows installers`.
После ручного запуска через **Actions → Windows installers → Run workflow** оба
установщика можно скачать одним artifact `camunda-support-windows-1.0.0`.

Windows-пакеты пока не подписываются сертификатом, поэтому при первом запуске
Windows SmartScreen может показать предупреждение.

## Документация

- [Продуктовая спецификация](docs/product-spec.md)
- [Дорожная карта и текущий статус](docs/roadmap.md)
- [Дизайн-система и иерархия UX](docs/design-system.md)

## Технические границы

- Kotlin Multiplatform + Compose Multiplatform;
- desktop JVM target для macOS и Windows;
- прямое обращение к Camunda 7 REST;
- локальное хранение метаданных;
- в дальнейшем — OS Keychain/Credential Manager для авторизации;
- WebView используется только как изолированный renderer `bpmn-js`; бизнес-UI остаётся нативным;
- без backend и серверной базы данных.
