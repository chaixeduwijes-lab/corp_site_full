# messenger-relay

Ciphertext-only релей для закрытого E2EE-мессенджера на группу до 10 человек.
Реализация backend по спецификации из
[`../docs/anonymous-vps-e2ee-messenger.md`](../docs/anonymous-vps-e2ee-messenger.md)
(§7, §19). Разбор слабых мест и границ гарантий —
[`../docs/security-audit-messenger-relay.md`](../docs/security-audit-messenger-relay.md).

Референсный E2EE-клиент, который реально шифрует поверх этого реле
(vodozemac: X3DH + Double Ratchet), — в [`client/`](client/README.md). Это
Cargo workspace: `cargo test` собирает и реле, и клиент.

## Модель безопасности

Сервер спроектирован как **потенциально скомпрометированный с первого дня**
(документ, §8). Атакующий с root, снапшотом диска и дампом БД получает только:

- opaque device id (UUID) и **публичные** Ed25519-ключи устройств;
- зашифрованные конверты (ciphertext) в RAM — и то лишь те, что ещё не
  доставлены и не старше TTL.

Гарантии реализации:

| Принцип (док.)                  | Реализация                                                                  |
|---------------------------------|-----------------------------------------------------------------------------|
| Ciphertext only (§7, §9)        | Сервер никогда не разбирает конверт — это opaque-байты E2EE-слоя клиентов   |
| Нет истории на сервере (§10)    | Очередь сообщений живёт **только в RAM**; удаление по ACK или TTL           |
| TTL ≤ 20 минут (§10)            | `MSGR_MESSAGE_TTL_SECS` жёстко ограничен потолком 1200 с в коде             |
| Ничего на диск (§8)             | Очередь не пишется на диск; держится `mlockall`+запретом core dump (см. ниже)|
| Best-effort wipe (§11)          | Буферы ciphertext в `Zeroizing` — зануляются при ACK/TTL/teardown           |
| Ключи не покидают устройств (§7)| Сервер хранит только публичные identity-ключи                               |
| Минимальные логи (§15)          | В `info` нет идентификаторов; присутствие — только `debug`                  |
| Закрытая группа                 | Регистрация только по pre-shared invite token + лимиты устройств/очереди    |

**Что сервер НЕ гарантирует (честно, см. аудит):**

- Это **zero-content-knowledge, не zero-knowledge.** Сервер видит
  маршрутные метаданные: отправителя (аутентифицирован), получателя (`to`),
  тайминги, размеры. Минимизация метаданных (sealed sender, padding) — future
  work (аудит F3).
- **«Ничего на диск» держится только при харднинге.** Без `mlockall` и
  запрета core dump ядро может вынести ciphertext/TLS-ключ в swap или дамп.
  Реле включает это само (`MSGR_MEMORY_HARDENING=true`), но нужны права
  (`CAP_IPC_LOCK`/`LimitMEMLOCK`) **и** выключенный/шифрованный swap на хосте.
- **Настоящая crypto-erasure — на клиентах** (ратчет выбрасывает message key).
  Реле лишь минимизирует время жизни ciphertext в RAM, не заменяет ратчет.
- Реле доверенно по **доступности** — злонамеренное реле может дропать
  сообщения; клиентам нужны end-to-end ACK и счётчики порядка (аудит F13).

E2EE-слой (X3DH/Double Ratchet или иная схема) — ответственность клиентов;
релей лишь переносит конверты. Плановая последовательность: сначала
качественный E2EE на клиентах, релей остаётся глупым (§18).

## Сборка и запуск

```bash
cd messenger
cargo test          # юнит- и интеграционные тесты
cargo build --release

MSGR_INVITE_TOKEN='<длинный случайный секрет>' \
MSGR_BIND_ADDR=127.0.0.1:8080 \
./target/release/messenger-relay
```

### Переменные окружения

| Переменная                  | По умолчанию     | Назначение                                              |
|-----------------------------|------------------|---------------------------------------------------------|
| `MSGR_BIND_ADDR`            | `127.0.0.1:8080` | Адрес прослушивания                                     |
| `MSGR_DB_PATH`              | `devices.db`     | SQLite с device id + публичными ключами (и только ими)  |
| `MSGR_INVITE_TOKEN`         | — (нет)          | Invite token; **не задан → регистрация закрыта**        |
| `MSGR_MESSAGE_TTL_SECS`     | `1200`           | TTL очереди; в коде ограничен диапазоном 10…1200 с      |
| `MSGR_MAX_QUEUE_PER_DEVICE` | `64`             | Лимит очереди на устройство                             |
| `MSGR_MAX_DEVICES`          | `16`             | Лимит устройств (10 человек × несколько устройств)      |
| `MSGR_MAX_BODY_BYTES`       | `16384`          | Лимит тела запроса                                      |
| `MSGR_MAX_TOTAL_QUEUE_BYTES`| `67108864`       | Глобальный потолок RAM под очередь (64 MiB) → `503` при превышении |
| `MSGR_MEMORY_HARDENING`     | `true`           | `mlockall` + запрет core dump при старте (unix)         |
| `MSGR_TLS_CERT` / `MSGR_TLS_KEY` | — (нет)     | PEM-серт и ключ → TLS через rustls; иначе plaintext     |

Дефолты рассчитаны на ~1.5 GB privacy-VPS (Njalla/FlokiNET): худший случай
per-device 16 KiB × 64 = 1 MiB, глобальный потолок 64 MiB — сильно ниже RAM
хоста (аудит F6).

Без `MSGR_TLS_*` сервер слушает открытый HTTP — используйте это только за
локальным reverse-proxy, терминирующим TLS 1.3, либо задайте оба параметра.

### Харднинг памяти (обязательно для гарантии «ничего на диск», аудит F1)

Реле при старте вызывает `mlockall(MCL_CURRENT|MCL_FUTURE)` (страницы не
уходят в swap) и `setrlimit(RLIMIT_CORE,0)` + `prctl(PR_SET_DUMPABLE,0)` (нет
core dump, нет `ptrace`/`/proc/pid/mem` от не-root). Это **best-effort**: при
нехватке прав пишется `warn` и реле продолжает работу, но гарантия диска не
держится. Для прод-деплоя используйте [`deploy/messenger-relay.service`](deploy/messenger-relay.service)
(`LimitMEMLOCK=infinity`, `LimitCORE=0`, sandbox-директивы) **и** отключите
или зашифруйте swap на хосте.

## Аутентификация устройств

У каждого устройства есть Ed25519 identity-ключ. Приватная часть никогда не
передаётся серверу.

**HTTP** — заголовки `x-device-id`, `x-timestamp` (unix-секунды),
`x-signature` (base64). Подписывается каноническая строка:

```text
v1|{METHOD}|{PATH}|{timestamp}|{sha256_hex(body)}
```

`PATH` — литеральный путь маршрута, без query string. Окно допустимого
расхождения часов ±300 с; повтор той же подписи в пределах окна отклоняется
(replay cache).

**WebSocket** — challenge-response: сервер шлёт случайный nonce, клиент
отвечает подписью над `ws-auth|v1|{nonce}`.

## API

### `POST /v1/register`

Регистрация нового устройства (закрыта, если invite token не задан).

```json
{
  "invite_token": "…",
  "identity_pk": "<base64 Ed25519 pk, 32 байта>",
  "signature": "<base64 подпись над 'register|v1|{invite_token}|{identity_pk}'>"
}
```

→ `201 {"device_id": "...", "created_at": …}`. Ошибки: `403
registration_disabled | invalid_invite_token`, `409 duplicate_identity_key |
device_limit_reached`.

### `GET /v1/devices` (подписанный)

Справочник публичных ключей группы:
`{"devices":[{"device_id","identity_pk","created_at"}]}`.

### `POST /v1/messages` (подписанный)

```json
{ "to": "<device_id получателя>", "ciphertext": "<base64 opaque конверт>" }
```

→ `202 {"id": "...", "expires_at": …}`. Сообщение попадает в RAM-очередь
получателя и живёт до ACK или TTL. Ошибки: `404 unknown_recipient`,
`429 queue_full`, `503 server_busy` (глобальный бюджет RAM исчерпан).

### `POST /v1/prekeys` (подписанный)

Публикация prekey-бандла устройства для асинхронного X3DH-first-contact.
Только **публичные** ключи; директорий живёт в RAM (см. `prekeys.rs`).

```json
{
  "identity_key": "<base64 Curve25519 identity>",
  "one_time_keys": [{"id": "...", "key": "<base64 Curve25519>"}],
  "fallback_key": "<base64 Curve25519 | null>"
}
```

→ `204`. Identity/fallback заменяются, one-time keys дописываются (dedup по id).

### `GET /v1/prekeys/{device_id}` (подписанный)

Claim бандла получателя, **потребляя один** one-time key (path включает
device_id и входит в подпись):
`{"identity_key", "one_time_key": {...}|null, "fallback_key": ...|null}`.
При исчерпании one-time keys возвращается переиспользуемый fallback. Ошибка:
`404 no_prekeys`.

### `GET /v1/ws` (WebSocket)

```text
S→C  {"type":"challenge","nonce":"<b64>"}
C→S  {"type":"auth","device_id":"…","signature":"<b64>"}
S→C  {"type":"ready","pending":N}
S→C  {"type":"message","id":"…","ciphertext":"<b64>","queued_at":…,"expires_at":…}
C→S  {"type":"ack","id":"…"}        # сервер удаляет сообщение
```

Недоставленные (без ACK) сообщения повторно отправляются при переподключении.
Новое подключение того же устройства вытесняет старое
(`{"type":"error","reason":"replaced"}`).

## Деплой (кратко)

Целевая площадка — privacy-oriented VPS (Njalla / FlokiNET, см. документ).
Минимальный hardening из §13–§15: отдельный SSH-ключ, `PasswordAuthentication
no`, firewall (наружу только 22 и 443), отдельный UNIX-пользователь для
сервиса, автообновления безопасности, **никаких** бэкапов очереди сообщений.

## Ограничения MVP (осознанные)

- Rate limiting не реализован — группа закрыта invite-токеном и лимитами
  очереди/устройств; при необходимости добавится на уровне reverse-proxy.
- Ротации/отзыва устройств пока нет (удаление строки из SQLite вручную).
- Референсный клиент и E2EE-библиотека клиента — следующий этап.
