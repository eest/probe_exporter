# probe_exporter

Playing around with a prometheus exporter for the MQTT messages sent by https://github.com/eest/esp32c6-probe

To use the code copy `probe_exporter.toml.sample` to `probe_exporter.toml` and fill in real values followed by `cargo run`.

The code will open a prometheus scraping endpoint listening at 127.0.0.1:9000
and updating the returned metrics as MQTT messages are processed.
