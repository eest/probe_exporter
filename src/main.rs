use axum::{Router, routing::get};
use metrics_exporter_prometheus::PrometheusBuilder;
use rumqttc::{AsyncClient, Event, MqttOptions, Packet, QoS};
use serde::Deserialize;
use std::fs;
use std::future::ready;
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

    let handle = builder
        .install_recorder()
        .expect("failed to install recorder");

    let upkeep_handle = handle.clone();
    tokio::spawn(async move {
        let upkeep_timeout = Duration::from_secs(5);
        loop {
            tokio::time::sleep(upkeep_timeout).await;
            upkeep_handle.run_upkeep();
        }
    });

    // build our application with a single route
    let app = Router::new().route("/metrics", get(move || ready(handle.render())));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:9000")
        .await
        .unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

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
