# ESP32 HTTP Server Demo with BME280 Sensor
This is a simple demonstration of an HTTP server running on the ESP32, featuring real-time data display from a BME280 sensor. The server hosts an `index.html` page that presents temperature, pressure, and humidity readings.

## Tech Stack
This example includes the following components:
- <b>ESP32 with `esp_idf_svc`:</b> Uses the community-maintained `esp_idf_svc` crate, which provides a built-in HTTP server.
- <b>Wi-Fi Connectivity:</b> Connects to a network using provided `WIFI_SSID` and `WIFI_PASS` credentials.
- <b>HTTP Server:</b> Utilizes the `esp_idf_svc::http` module to serve web content.
- <b>BME280 Sensor:</b> Measures temperature, pressure, and humidity.
- <b>Web Interface:</b> Serves an `index.html` page at `/` to display sensor data.
- <b>Multithreading:</b> Leverages `std::thread` to concurrently manage sensor readings and HTTP server operations.
