//
// Copyright (C) 2016-2020 Abstract Horizon
// All rights reserved. This program and the accompanying materials
// are made available under the terms of the Apache License v2.0
// which accompanies this distribution, and is available at
// https://www.apache.org/licenses/LICENSE-2.0
//
//  Contributors:
//    Daniel Sendula - initial API and implementation
//

#![macro_use]

use std::io::prelude::*;
use std::net::{TcpStream, TcpListener};
use std::{thread, sync::Arc};
use std::sync::mpsc;
use byteorder::{ByteOrder, LittleEndian};

// use crate::telemetry_stream::{TelemetryStreamDefinition, TelemetryStreamField, FieldType, FieldTypeUnsignedByte};
use crate::telemetry_stream::*;


pub struct SocketTelemetryServerBuilder {
    stream_definitions: Vec<Vec<u8>>
}

impl SocketTelemetryServerBuilder {
    pub fn new() -> SocketTelemetryServerBuilder {
        SocketTelemetryServerBuilder {
            stream_definitions: vec![]
        }
    }

    pub fn register_stream(&mut self, stream: TelemetryStreamDefinition) -> TelemetryStreamDefinition {
        self.stream_definitions.push(stream.to_json().into_bytes());
        stream
    }

    pub fn create(self, port: u16) -> SocketTelemetryServer {
        SocketTelemetryServer::new(port, Arc::new(self.stream_definitions.clone()))
    }
}

pub struct SocketTelemetryServer {
    port: u16,
    log_sender: mpsc::Sender<Vec<u8>>,
    stop_log_sender: mpsc::Sender<bool>,
    stop_con_sender: mpsc::Sender<bool>,
    con_thread: thread::JoinHandle<()>,
    log_thread: thread::JoinHandle<()>
}

impl SocketTelemetryServer {
    pub fn new(port: u16, streams: Arc<Vec<Vec<u8>>>) -> SocketTelemetryServer {
        let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).unwrap();

        let (log_tx, log_rx) = mpsc::channel();
        let (con_tx, con_rx) = mpsc::channel();
        let (stop_log_tx, stop_log_rx) = mpsc::channel();
        let (stop_con_tx, stop_con_rx) = mpsc::channel();

        SocketTelemetryServer {
            port,
            log_sender: log_tx,
            stop_log_sender: stop_log_tx,
            stop_con_sender: stop_con_tx,
            con_thread: thread::spawn(move || {
                for stream in listener.incoming() {
                    match stop_con_rx.try_recv() {
                        Ok(_) => break,
                        _ => {}
                    };

                    match stream {
                        Ok(stream) => {
                            println!("Received new connection...");
                            con_tx.send(stream).unwrap();
                        }
                        _ => {}
                    }
                }
                println!("Finishing connection thread.");
            }),
            log_thread: thread::spawn(move || {
                let mut connections: Vec<TcpStream> = vec![];
                for log_message in log_rx.iter() {
                    match stop_log_rx.try_recv() {
                        Ok(_) => break,
                        _ => {}
                    };

                    // println!("Received log {}", log);
                    for connection in con_rx.try_iter() {
                        // println!("   and received new connection, sending streams back {}", streams[0].to_json());
                        let mut con = &connection;
                        // let _ = con.write(b"STRS");
                        let mut buf = [0u8; 8];
                        buf[0..4].clone_from_slice("STRS".as_bytes());
                        LittleEndian::write_u32(&mut buf[4..], streams.len() as u32);
                        let _ = con.write(&buf);
                        // println!("Sent out {:?}", buf);

                        for stream_definition in streams.iter(){
                            // let _ = con.write(b"STDF");
                            let mut buf = [0u8; 8];
                            buf[0..4].clone_from_slice("STDF".as_bytes());
                            LittleEndian::write_u32(&mut buf[4..], stream_definition.len() as u32);
                            let _ = con.write(&buf);
                            // println!("Sent out {:?}", buf);
                            let _ = con.write(stream_definition);
                        }
                        connections.push(connection);
                    }

                    for mut connection in connections.iter() {
                        let con = &mut connection;
                        // println!("Should send logged statement here to the connection...");
                        // let _ = con.write(log.to_string().as_bytes());
                        let _ = con.write(&log_message);
                    }
                }
                println!("Finishing logging thread.");
            })
        }
    }

    pub fn stop(self) {
        let _ = self.stop_log_sender.send(true);
        let _ = self.stop_con_sender.send(true);
        let _ = self.log_sender.send(vec![]);

        let mut stream = TcpStream::connect(format!("127.0.0.1:{}", self.port)).unwrap();

        let _ = stream.write(&[1]);

        let _ = self.log_thread.join();
        let _ = self.con_thread.join();
    }

    pub fn log(&self, buf: Vec<u8>) {
        self.log_sender.send(buf).unwrap();
    }
}

#[macro_export]
macro_rules! log_with_time {
    ( $logger: expr, $stream: expr, $( $value:expr ),* ) => {
        {
            let mut buf: Vec<u8> = Vec::with_capacity($stream.size());

            let start = SystemTime::now();
            let since_the_epoch = start.duration_since(UNIX_EPOCH).expect("Time went backwards");
            let now = since_the_epoch.as_secs_f64();

            $stream.write_header(&mut buf);
            now.store(&mut buf);

            let mut fields = $stream.fields();
            let mut i = 0;
            $(
                i = i + 1;
                match fields.next() {
                    Some(_field) => {
                        $value.store(&mut buf);
                    },
                    None => {
                        panic!("Too many parameters {}", i);
                    }
                }
            )*
            loop {
                match fields.next() {
                    Some(field) => {
                        panic!("Unsatisfied field {}", field.name());
                    },
                    None => break
                }
            }
            if buf.len() < $stream.size() {
                println!("Underallocated buf, needed {}, but was only {}", $stream.size(), buf.len()); // TODO error
                buf.resize($stream.size(), 0);
            } else if buf.len() > $stream.size() {
                panic!("Error: buffer too big, expected {}, but as {}", $stream.size(), buf.len());
            }

            $logger.log(buf);
        }
    };
}


#[macro_export]
macro_rules! log {
    ( $logger: expr, $stream: expr, $time:expr, $( $value:expr ),* ) => {
        {
            let mut buf: Vec<u8> = Vec::with_capacity($stream.size());

            $stream.write_header(&mut buf);
            $time.store(&mut buf);

            let mut fields = $stream.fields();
            let mut i = 0;
            $(
                i = i + 1;
                match fields.next() {
                    Some(_field) => {
                        $value.store(&mut buf);
                    },
                    None => {
                        panic!("Too many parameters {}", i);
                    }
                }
            )*
            loop {
                match fields.next() {
                    Some(field) => {
                        panic!("Unsatisfied field {}", field.name());
                    },
                    None => break
                }
            }
            if buf.len() < $stream.size() {
                println!("Underallocated buf, needed {}, but was only {}", $stream.size(), buf.len()); // TODO error
                buf.resize($stream.size(), 0);
            } else if buf.len() > $stream.size() {
                panic!("Error: buffer too big, expected {}, but as {}", $stream.size(), buf.len());
            }

            $logger.log(buf);
        }
    };
}
