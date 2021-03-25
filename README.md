# DelayMapI

The API for [DelayMap](https://github.com/Robbe7730/DelayMap) 3.0.0.

## Installation

1. Make sure you have `protoc` installed
2. Cargo run --release

## API format

```rust
struct DelayMapStopTime {
    id: String;
    name: String,
    lat: Option<f64>,
    lon: Option<f64>,
    arrival_delay: i32,         // In seconds
    arrival_timestamp: u32,     // In seconds after midnight
    departure_delay: i32,       // In seconds
    departure_timestamp: u32,   // In seconds after midnight
}

struct DelayMapTrain {
    name: String,
    stops: Vec<DelayMapStopTime>,
    stop_index: usize,
    is_stopped: bool,
    estimated_lat: f64,
    estimated_lon: f64,
    stop_id: String,
}
```
