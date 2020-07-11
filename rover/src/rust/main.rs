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

use balance::{Balance, BalanceControl, ConfigData};

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

    fn subscribe_storage(&mut self, topic: &'static str, callback: fn(msg: mqtt311::Publish, mqtt_client: &mut MQTTClient) -> ()) {
        self.mqtt_client.subscribe(&("storage/write/".to_string() + topic), QoS::AtMostOnce).unwrap();
        let _ = self.mqtt_client.publish(&("storage/read/".to_string() + topic), QoS::AtLeastOnce, false, "");
        self.subscriptions.insert(Box::leak(("storage/write/".to_string() + topic).into_boxed_str()), callback);
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

fn float_payload(msg: mqtt311::Publish, mqtt_client: &mut MQTTClient, update: fn(&mut ConfigData, f: f64) -> ()) {
    match String::from_utf8(msg.payload.to_vec()) {
        Ok(s) => match s.parse() {
            Ok(f) => {
                // println!("Got combine_gyro_factor {}", f);
                update(&mut mqtt_client.balance_control.config_data, f);
                mqtt_client.balance_control.send_config();
            },
            _ => println!("Failed to parse {} for  {}", s, msg.topic_name)
        },
        _ => println!("Failed to convert to utf8 {:?} for  {}", msg.payload, msg.topic_name)
    }
}


fn main() {
    match MqttClient::start(MqttOptions::new("balance-r", "172.24.1.174", 1883).set_keep_alive(10)) {
        Ok((mqtt_client, notifications)) => {

            let balance = Balance::new();

            let balance_control = balance.start();

            let mut mqtt_client = MQTTClient::new(mqtt_client, balance_control);

            mqtt_client.subscribe_storage("balance/gyro/filter", |msg, mqtt_client| 
                float_payload(msg, mqtt_client, |config_data, f| config_data.combine_gyro_factor = f)
            );
            mqtt_client.subscribe_storage("balance/accel/filter", |msg, mqtt_client|
                float_payload(msg, mqtt_client, |config_data, f| config_data.combine_accel_factor = f)
            );
            mqtt_client.subscribe_storage("balance/combine_factor_gyro", |msg, mqtt_client|
                float_payload(msg, mqtt_client, |config_data, f| config_data.combine_gyro_accel_factor = f)
            );
            mqtt_client.subscribe_storage("balance/pid_inner/p", |msg, mqtt_client|
                float_payload(msg, mqtt_client, |config_data, f| config_data.pid_kp = f)
            );
            mqtt_client.subscribe_storage("balance/pid_inner/i", |msg, mqtt_client|
                float_payload(msg, mqtt_client, |config_data, f| config_data.pid_ki = f)
            );
            mqtt_client.subscribe_storage("balance/pid_inner/d", |msg, mqtt_client|
                float_payload(msg, mqtt_client, |config_data, f| config_data.pid_kd = f)
            );
            mqtt_client.subscribe_storage("balance/pid_inner/g", |msg, mqtt_client|
                float_payload(msg, mqtt_client, |config_data, f| config_data.pid_gain = f)
            );
            mqtt_client.subscribe("storage/write/balance/pid_outer/p", |_msg, _mqtt_client| {});
            mqtt_client.subscribe("storage/write/balance/pid_outer/i", |_msg, _mqtt_client| {});
            mqtt_client.subscribe("storage/write/balance/pid_outer/d", |_msg, _mqtt_client| {});
            mqtt_client.subscribe("storage/write/balance/pid_outer/g", |_msg, _mqtt_client| {});

            mqtt_client.subscribe("balancing/calibrate", |_, mqtt_client| {
                mqtt_client.balance_control.calibrate();
            });
            mqtt_client.subscribe("balancing/start", |_, mqtt_client| {
                mqtt_client.balance_control.start_balancing();
            });
            mqtt_client.subscribe("balancing/stop", |_, mqtt_client| {
                mqtt_client.balance_control.stop_balancing();
            });
            // mqtt_client.subscribe("balancing/request-info", |_, mqtt_client| {});

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
