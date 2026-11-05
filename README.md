# Camunda Support Desktop

Нативное desktop-приложение для специалистов поддержки, которые расследуют и восстанавливают процессы Camunda 7 на macOS, Windows и Ubuntu.

Приложение работает полностью на клиенте: напрямую обращается к настроенным Camunda REST endpoints и не требует backend-сервиса.

С версии 2.3 интерфейс написан на Rust без веба: [GPUI](https://github.com/zed-industries/zed/tree/main/crates/gpui) (движок Zed, рендер через Metal / DirectX / Vulkan) и компоненты [GPUI Kit](https://gpui-kit.com). Почему выбран этот стек — в [исследовании](docs/research/Нативный%20Rust%20UI%20для%20Camunda%20Support.md). Подключения и тема из версий 1.x и 2.x переносятся автоматически.

## Что умеет

- **Обзор**: инциденты, заявки в работе и зависшие дольше 4 часов, группы ошибок по кубику и подписи сообщения, кубики с самым долгим ожиданием; процессы свёрнуты по ключу, внутри версии с датой деплоя, сначала новые;
- **Поиск** заявки по бизнес-ключу или ID и по переменным процесса с каталогом ключей;
- **Инциденты**: группы и список с фильтрами по периоду, процессу, типу и тексту, массовый повтор;
- **Заявка**: схема BPMN с подсветкой пройденных, активных и аварийных кубиков, зум и пан; **клик по кубику открывает телепорт** в него; хронология, диагностика с повтором заданий и внешних задач, переменные с поиском и правкой, метаданные, приостановка и активация, ссылка `camunda-support://`, отчёт в Markdown, переход в Cockpit;
- **⌘K**: поиск заявки и переходы по разделам; ⌘1–3 и ⌘, — разделы;
- **Настройки**: подключения с проверкой `/engine-rest`, светлая и тёмная тема, обновления с GitHub Releases со сверкой SHA-256;
- шрифты GitLab Sans и GitLab Mono, палитра Pajamas.

## Структура

```text
crates/camunda-core/   клиент Camunda 7 REST, хранилище, миграция 1.x, deep links, обновления
native/                приложение на GPUI Kit
  src/                 экраны: overview, search, incidents, definition, instance, settings, palette
  assets/              шрифты GitLab, иконки
  scripts/             упаковка macOS (.app, .dmg) и Linux (.deb)
  packaging/windows/   NSIS-установщик
scripts/               релиз и запись версии
```

## Разработка

Нужен Rust 1.95+ (через rustup). На macOS нужен Xcode для компиляции шейдеров Metal.

```bash
cd native
cargo run
cargo test
cargo test --manifest-path ../crates/camunda-core/Cargo.toml
```

Переменные для отладки: `CAMUNDA_SUPPORT_CONNECTION=<имя подключения>` выбирает подключение при старте, `CAMUNDA_SUPPORT_OPEN_INSTANCE=<id>` сразу открывает заявку.

Пакеты локально:

```bash
native/scripts/package-macos.sh   # target/release/bundle/dmg/*.dmg
native/scripts/package-linux.sh   # target/release/bundle/deb/*.deb (на Ubuntu)
```

## Релиз

```bash
./scripts/release.sh v2.3.0
```

Скрипт гоняет тесты, записывает версию в `native/Cargo.toml`, коммитит и пушит тег. GitHub Actions собирает установщики для Windows (`.exe`), macOS ARM64 (`.dmg`) и Ubuntu (`.deb`) и публикует релиз с `SHA256SUMS.txt`. Имена ассетов совместимы с прежними версиями, поэтому старые клиенты увидят обновление.

Пакеты не подписаны. На macOS при первом запуске откройте приложение через «Системные настройки → Конфиденциальность и безопасность → Всё равно открыть».

## Диплинки

Кнопка копирования ссылки на экране заявки даёт адрес вида:

```text
camunda-support://process-instance/INSTANCE_ID?origin=https%3A%2F%2Fcamunda.example.kz
```

Протокол регистрирует установщик (Info.plist на macOS, реестр на Windows, `.desktop` на Linux). Приложение сопоставляет домен с сохранённым подключением и открывает заявку.
