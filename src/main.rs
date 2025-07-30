use metrics_exporter_prometheus::PrometheusBuilder;
use rumqttc::{AsyncClient, Event, MqttOptions, Packet, QoS};
use serde::Deserialize;
use std::fs;
use std::time::Duration;
use toml::Table;

#[derive(Deserialize, Debug)]
struct ProbeData {
    centigrade: f32,
    humidity_percent: f32,
}

#[tokio::main]
async fn main() {
    let toml_string: String = fs::read_to_string("probe_exporter.toml").unwrap();

    let toml_conf = toml_string.parse::<Table>().unwrap();

    let mqtt_client_id = toml_conf["mqtt_client_id"].as_str().unwrap();
    let mqtt_server_host = toml_conf["mqtt_server_host"].as_str().unwrap();
    let mqtt_username = toml_conf["mqtt_username"].as_str().unwrap();
    let mqtt_password = toml_conf["mqtt_password"].as_str().unwrap();

    let mut mqttoptions = MqttOptions::new(mqtt_client_id, mqtt_server_host, 1883);
    mqttoptions.set_keep_alive(Duration::from_secs(5));

    mqttoptions.set_credentials(mqtt_username, mqtt_password);

    let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);
    client
        .subscribe("temperature/1", QoS::AtMostOnce)
        .await
        .unwrap();

    let builder = PrometheusBuilder::new();
    builder
        .install()
        .expect("failed to install recorder/exporter");

    let counter = metrics::counter!("mqtt_pub_counter", "sensor" => "sensor1");
    let centigrade_gauge = metrics::gauge!("centigrade", "sensor" => "sensor1");
    let humidity_percent_gauge = metrics::gauge!("humidity_percent", "sensor" => "sensor1");

    loop {
        let res = eventloop.poll().await;
        match res {
            Ok(notification) => {
                println!("Received = {notification:?}");
                if let Event::Incoming(Packet::Publish(p)) = notification {
                    counter.increment(1);
                    let deserialized: ProbeData = serde_json::from_slice(&p.payload).unwrap();
                    println!("{deserialized:?}");
                    centigrade_gauge.set(deserialized.centigrade);
                    humidity_percent_gauge.set(deserialized.humidity_percent);
                }
            }
            Err(conn_err) => {
                println!("connection error: {conn_err}");
                break;
            }
        }
    }
}
