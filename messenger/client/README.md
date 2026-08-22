# messenger-client

Референсный E2EE-клиент для [`messenger-relay`](../README.md). Показывает
целевое свойство всей системы: **plaintext существует только на устройствах,
реле переносит opaque-байты**.

## Крипто — не своё

Вся криптография делегирована [`vodozemac`](https://crates.io/crates/vodozemac)
(аудированная реализация Olm: X3DH-подобный handshake + Double Ratchet). Это
прямое следствие вывода аудита «не изобретай крипту» — клиент не содержит
самописных примитивов.

Используются **две независимые пары ключей**:

- **Ed25519 transport key** (`ed25519-dalek`) — аутентифицирует HTTP/WebSocket
  запросы к реле (та же каноническая строка, что и на сервере — переиспользуем
  `messenger_relay::auth`, чтобы клиент и сервер не разошлись).
- **vodozemac account** — messaging-идентичность + состояние ратчета.

Приватные части обеих пар **никогда не покидают клиент**. Реле видит только
transport public key и публичный prekey-бандл.

## Поток

```text
register            → POST /v1/register        (подписано transport key)
publish_prekeys(n)  → POST /v1/prekeys         identity + n one-time + fallback
send_text(peer,txt) → GET  /v1/prekeys/{peer}  (claim бандла при первом контакте)
                      создать outbound-сессию (X3DH)
                      session.encrypt → pack → base64
                    → POST /v1/messages         {to, ciphertext}
connect()           → WS /v1/ws                 challenge-response по transport key
recv()              ← message                   unpack → decrypt (ратчет) → ACK
```

Отправитель в кадре реле **не передаётся** — получатель извлекает identity
отправителя из самого pre-key сообщения (Olm), реле его не сообщает.

## API (набросок)

```rust
let mut alice = Client::register("http://127.0.0.1:8080", invite).await?;
let mut bob   = Client::register("http://127.0.0.1:8080", invite).await?;
bob.publish_prekeys(10).await?;

alice.send_text(&bob.device_id, "привет").await?;

bob.connect().await?;
let msg = bob.recv().await?.unwrap();
assert_eq!(msg.plaintext, "привет");
assert_eq!(msg.sender_identity, alice.identity_key());
```

## Тесты

```bash
cargo test -p messenger-client
```

- `plaintext_never_reaches_the_relay` — поднимает **настоящее** реле на TCP,
  Alice шлёт секрет Bob'у; тест читает то, что реле реально держит в очереди, и
  доказывает: ровно один конверт, **без подстроки plaintext**, но валидный
  Olm-message (реле его не понимает). Затем Bob расшифровывает end-to-end, ACK
  удаляет сообщение.
- `bidirectional_conversation` — полный обмен в обе стороны с продвижением
  ратчета (pre-key → normal, два раунда).

## Ограничения референса

- Ключи держатся только в памяти (нет pickle/persist между запусками) — это
  демонстрационный клиент, не end-user приложение.
- Групповые сообщения — попарные сессии (для ≤10 человек достаточно; Meg4M/
  sender-keys не реализованы).
- Верификация identity (safety numbers) и отзыв устройств — следующий слой.
