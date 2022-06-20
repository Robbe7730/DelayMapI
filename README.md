# DelayMapI

[![codecov](https://codecov.io/gh/Robbe7730/DelayMapI/branch/main/graph/badge.svg?token=LQ1YP7VE7O)](https://codecov.io/gh/Robbe7730/DelayMapI)

The API for [DelayMap](https://github.com/Robbe7730/DelayMap) 3.0.0.

## Installation

1. Make sure you have `protoc` installed
2. Cargo run --release

## API format

All endpoints accept the url parameter `language` for the following languages:

- English (`language=en`, default)
- Dutch (`language=nl`)
- French (`language=fr`)
- German (`language=de`)

### /trains

Returns a list of `DelayMapTrain` with the following structures:


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
    stop_id: String,
}

struct DelayMapTrain {
    name: String,
    stops: Vec<DelayMapStopTime>,
    stop_index: usize,
    is_stopped: bool,
    estimated_lat: f64,
    estimated_lon: f64,
}
```

### /works

Returns a list of `DelayMapWorks` with the following structures:

```rust
struct DelayMapStop {
    name: String,
    lat: Option<f64>,
    lon: Option<f64>,
    stop_id: String,
}

struct DelayMapURL {
    url: String,
    label: String,
}

struct DelayMapWorks {
    id: String,
    name: String,
    message: String,
    impacted_station: Option<DelayMapStop>,
    start_date: String,
    end_date: String,
    start_time: String,
    end_time: String,
    urls: Vec<DelayMapURL>,
}
```
