//
// Copyright (C) 2020 Abstract Horizon
// All rights reserved. This program and the accompanying materials
// are made available under the terms of the Apache License v2.0
// which accompanies this distribution, and is available at
// https://www.apache.org/licenses/LICENSE-2.0
//
//  Contributors:
//    Daniel Sendula - initial API and implementation
//


mod telemetry_stream;

mod telemetry_socket_server;

mod pid;
mod motors;
mod balance;
mod gyro;
mod accel;

use balance::{Balance, BalanceControl};

use std::collections::HashMap;
//use std::time::Duration;
//use std::thread;

use crossbeam_channel::select;
use ctrlc;

use rumqtt::{MqttClient, MqttOptions, QoS, Notification};
use mqtt311;



struct MQTTClient {
    mqtt_client: MqttClient,
    subscriptions: HashMap<&'static str, fn(msg: mqtt311::Publish, mqtt_client: &mut MQTTClient)>,
    balance_control: BalanceControl,
}

impl MQTTClient {
    fn new(mqtt_client: MqttClient, balance_control: BalanceControl) -> MQTTClient {
        MQTTClient {
            mqtt_client,
            subscriptions: HashMap::new(),
            balance_control,
        }
    }

    fn subscribe(&mut self, topic: &'static str, callback: fn(msg: mqtt311::Publish, mqtt_client: &mut MQTTClient) -> ()) {
        self.mqtt_client.subscribe(topic, QoS::AtMostOnce).unwrap();
        self.subscriptions.insert(topic, callback);
    }

    fn process(&mut self, notification: Notification) {
        match notification {
            Notification::Publish(msg) => {
                match self.subscriptions.get(&msg.topic_name.as_str()) {
                    Some(f) => f(msg, self),
                    _ => println!("Cannot find notification for topic {}", msg.topic_name)
                }
            },
            Notification::Reconnection => {
                for key in self.subscriptions.keys() {
                    let topic : &'static str = key;
                    let _ = self.mqtt_client.subscribe(topic, QoS::AtMostOnce);
                }
            },
            _ => { }
        }
    }
    
    fn stop(self) {
        self.balance_control.stop();
    }
}


fn main() {
    match MqttClient::start(MqttOptions::new("balance-r", "172.24.1.174", 1883).set_keep_alive(10)) {
        Ok((mqtt_client, notifications)) => {

            let balance = Balance::new();

            let balance_control = balance.start();

            let mut mqtt_client = MQTTClient::new(mqtt_client, balance_control);

            mqtt_client.subscribe("hello", |msg, mqtt_client| {
                println!("Received on {} msg {}", msg.topic_name, std::str::from_utf8(&msg.payload).unwrap());
                mqtt_client.balance_control.config_data.pid_kp = 0.72;
                mqtt_client.balance_control.send_config();
            });

            let (stop_sender, stop_receiver) = crossbeam_channel::bounded(1);

            ctrlc::set_handler(move || {
                let _ = stop_sender.send(true);
            }).expect("Error setting Ctrl-C handler");

            loop {
                select! {
                    recv(notifications) -> notification => {
                        println!("Received {:?}", notification);
                        match notification {
                            Ok(notification) => mqtt_client.process(notification),
                            _ => {}
                        }
                    }
                    recv(stop_receiver) -> _done => break
                }
            }

            println!("Finishing...");
            mqtt_client.stop();
            println!("Done.");
        }
        _ => println!("Failed to connect to mosquito broker on this host")
    }
}
