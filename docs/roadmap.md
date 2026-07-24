# Дорожная карта

Актуальное состояние: 17 июля 2026 года.

## Foundation — выполнено

Цель: зафиксировать границы продукта и получить запускаемый cross-platform desktop shell.

- [x] Общее описание и информационная архитектура
- [x] Дизайн-концепт и язык окружений
- [x] Kotlin Multiplatform с desktop JVM target
- [x] Compose Desktop application shell
- [x] Абстракция локального connection repository
- [x] Sidebar connections и навигация
- [x] Форма создания connection и валидация
- [x] Строгая проверка окончания REST URL на `/engine-rest`
- [x] Обязательный ping `/version` с отдельными ошибками DNS, timeout, TLS и HTTP
- [x] Автоматическое построение Cockpit URL из домена REST с ручным override
- [x] Светлая тема и полностью русский интерфейс
- [x] Unit-тесты и runtime-проверка macOS
- [ ] CI-сборки на macOS и Windows
- [ ] Стратегия code signing и распространения

## MVP 1 — Read-only расследование

Цель: специалист может найти процесс и понять причину зависания без ручных REST-запросов.

### Connections

- [x] Локально хранить метаданные connection
- [x] Проверять `/version` до сохранения и показывать версию движка
- [x] Автоматически генерировать Cockpit URL и разрешать override
- [x] Мигрировать старые connections без Cockpit URL
- [ ] Поддержать no-auth, Basic и Bearer, позднее mTLS
- [ ] Хранить secrets в Keychain/Credential Manager

### Connection dashboard

- [x] Загружать последние версии process definitions
- [x] Сортировать definitions по времени deployment
- [x] Показывать состояние, текущие instances и открытые incidents
- [x] Режимы «Список» и карточечное «Превью»
- [x] Фильтр по названию, key, ID и tenant ID
- [x] По умолчанию открывать definition внутри приложения
- [x] Оставить Cockpit только отдельной явной кнопкой; корневой маршрут — `#/dashboard`
- [x] Периоды «Сегодня», «Вчера» и произвольный диапазон
- [x] Для периода считать стартовавшие instances и созданные incidents через History API
- [x] Считать полностью завершённые instances за последние 24 часа, сегодня, вчера и произвольный период
- [x] Показывать динамику стартов, полных завершений и incidents по часам или дням
- [x] Показывать среднее время прохождения, долю завершения и сводку критичных процессов
- [x] Группировать incidents по типам и открывать их прямо с дашборда
- [x] Показывать топ BPMN-кубиков по количеству incidents
- [x] Показывать подробную hover-карточку процесса и явные переходы в процесс и incidents
- [x] Показывать расширенные сведения definition: version tag, TTL, resource, deployment и Tasklist state
- [x] Передавать выбранный период на экран процесса и показывать разные historic instances
- [x] BPMN через embedded `bpmn-js` с подсветкой активных элементов и incidents
- [ ] Нативные BPMN-превью в карточках дашборда
- [ ] Экспорт снимка dashboard в CSV/XLSX

### Process explorer

- [x] Искать runtime instances по любой variable key/value
- [x] Поддержать значения String, Number и Boolean
- [x] Сохранять variable keys локально отдельно для connection
- [x] Показывать найденные instances и открывать внутреннюю карточку заявки
- [ ] Искать runtime instances напрямую по business key
- [ ] Искать завершённые instances через History API
- [ ] Показывать несколько связанных parent/child process instances
- [ ] Показывать все версии process definition с отдельной статистикой

### Карточка процесса и заявки

- [x] Внутренний экран process definition со списком запущенных экземпляров
- [x] Метаданные process definition за кнопкой «Подробнее»
- [x] Основные идентификаторы и состояние заявки
- [x] Activity-instance tree и подсветка активного BPMN-элемента
- [x] Открытые incidents и сообщения ошибок
- [x] External tasks, worker и lock state
- [x] Engine jobs, retries и exception message
- [x] Process variables без десериализации Java-объектов
- [x] Безопасное изменение примитивных и JSON runtime variables с сохранением типа
- [x] Вкладки «Схема», «Диагностика», «Переменные», «Метаданные» вместо длинной страницы
- [ ] Полные stacktraces jobs и error details external tasks
- [ ] Local variables по activity/execution scope
- [ ] Хронологическая activity history

**Критерий выхода:** реальное окружение можно расследовать от application ID до причины зависания без изменяющих запросов.

## Ближайший план

### Итерация 1 — Углубление диагностики заявки

1. [x] Поиск по фактическим variables выбранного процесса с фильтруемым и межсессионным кэшем ключей.
2. Полные stacktraces заданий и error details внешних задач.
3. Просмотр local variables с копированием и маскированием.
4. История прохождения по activity и переходы в вызванные процессы.
5. User tasks, executions и связанные подпроцессы.

### Итерация 2 — Диагностика incidents

1. [x] Реальный общий экран открытых и исторических incidents с фильтрами по дням, периодам, версиям и переходом в заявку.
2. Групповые retries только для выбранной заявки после проверки состояния.
3. Настраиваемое число retries вместо фиксированного значения 3.
4. История activities и incidents по заявке.
5. Read-only отчёт, который можно приложить к обращению поддержки.

### Итерация 3 — Авторизация и безопасное распространение

1. Basic и Bearer authentication для connections.
2. Keychain macOS и Credential Manager Windows.
3. Проверка прав отдельно для read-only и mutation endpoints.
4. CI-сборки macOS/Windows, code signing и инструкция установки.
5. Маскирование чувствительных variables по правилам.

## MVP 2 — Безопасное восстановление

Цель: закрыть распространённые incidents без process modification.

- [x] Поднятие retries external task до 3;
- [x] Unlock с предупреждением о повторном выполнении;
- [x] Поднятие retries job до 3;
- синхронный execute job только в expert mode;
- [x] Suspend/activate process instance;
- correlation разрешённых messages;
- preview и повторная проверка текущего состояния;
- локальный audit;
- [x] Отдельное подтверждение для production;
- [ ] Обязательная причина для каждой изменяющей операции.

**Критерий выхода:** типовые failed external task и failed job восстанавливаются безопасно и оставляют audit trail.

## MVP 3 — Пошаговый телепорт

Цель: полностью убрать ручное редактирование modification curl.

- [x] Парсить BPMN XML и строить каталог activities;
- [x] Выбирать конкретный source activity instance;
- [x] Выбирать target по названию с отображением технического ID;
- [x] Искать target по названию, ID, типу и `camunda:topic`;
- поддержать `startBeforeActivity`, `startAfterActivity`, `startTransition`;
- [x] Поддержать `startBeforeActivity`;
- [x] Поддержать точную отмену activity instance;
- добавлять типизированные variables к start instruction;
- явно настраивать listeners и I/O mappings;
- предупреждать о cancellation propagation и multi-instance;
- [x] Показывать предварительный REST payload;
- повторно читать и сравнивать activity tree;
- показывать состояние до и после.

**Критерий выхода:** для разрешённых сценариев поддержка больше не редактирует modification JSON руками.

## MVP 4 — BPMN-визуализация

Цель: визуально показывать текущую и целевую activity через изолированный WebView.

- [x] Использовать официальный `bpmn-js` вместо собственного renderer;
- [x] Хранить JS/CSS локально для работы без доступа к CDN;
- [x] Zoom, pan и fit-to-screen;
- [x] Подсвечивать текущие активные элементы;
- [x] Подсвечивать incidents на диаграмме;
- выбирать source/target телепорта на диаграмме;
- отображать переходы в called processes;
- сохранять list/tree fallback для неподдержанных BPMN элементов.

## MVP 5 — Operational hardening

Цель: подготовить контролируемое внутреннее распространение.

- masking для ИИН, OTP, tokens и документов;
- import/export очищенных connection profiles;
- опциональный append-only audit sink без превращения приложения в web-систему;
- crash reports без бизнес-данных;
- update channel и подписанные релизы;
- offline-safe миграции локальной схемы;
- UI automation и Camunda contract tests;
- accessibility и keyboard navigation;
- инструкции установки Windows/macOS.

## После MVP

- restart завершённых или отменённых instances;
- migration между process definitions;
- утверждённые организацией operation recipes;
- аналитика частых incidents;
- массовые read-only отчёты;
- отдельно согласованные guarded batch operations.
