# Binance API Integration

Инструмент для сбора рыночных данных с криптовалютной биржи Binance. Поддерживает работу как с REST API, так и с WebSocket API, позволяя собирать снимки (snapshots), инкрементальные обновления (diff/depth) и данные о сделках (trades).

## Quick Start
-  **Сборка и запуск:**
   ```bash
   cargo build -p binance-api-integration
   target/release/binance-api-integration --runtime 1min --symbols-path symbols.txt
   ```
- **Получение справки:**
   ```bash
   binance-api-integration --help
   ```


## Описание

- **Снепшоты (snapshots):** Полные снимки состояния книги заявок. Получаются через REST API Binance с минимальным интервалом 1 час. Используются как базовые точки для анализа или исполнения.
  
- **Инкрементальные обновления (diff/depth):** Изменения стакана в реальном времени. Получаются через WebSocket API Binance.
  
- **Сделки (trades):** Данные о торговых операциях (купля/продажа). Получаются через WebSocket API Binance.

## Формат данных

Каждое событие представлено структурой `Event`:

```rust
pub struct Event {
    local_unique_id: i64,       // Локальный уникальный ID события
    venue_timestamp: i64,       // Метка времени с биржи
    gate_timestamp: i64,        // Метка времени приёма данных
    event_type: String,         // Тип события (например, snapshot, depth, trade)
    product: String,            // Продукт (пара, например BTCUSDT)
    id1: Option<u64>,           // Дополнительный идентификатор события
    id2: Option<u64>,           // Дополнительный идентификатор события
    ask_not_bid: Option<bool>,  // True, если это заявка на продажу (ask)
    buy_not_sell: Option<bool>, // True, если это покупка (buy)
    price: String,              // Цена
    quantity: String,           // Объём
}
```

## Результат работы

Все данные сохраняются в папку `marketdata` в бинарных файлах (`.bin`), именованные по дате и времени, с интервалом 1 час. Эти файлы содержат сериализованные объекты `Event`.

Пример:
```
marketdata/
├── 2024-11-25 10-00-00.bin
├── 2024-11-25 11-00-00.bin
└── ...
```