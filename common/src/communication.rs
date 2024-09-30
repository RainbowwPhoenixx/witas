use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc::{channel, Receiver, Sender};

use serde::{Deserialize, Serialize};
use tracing::error;

use crate::tas::{PlaybackState, TraceDrawOptions};

#[derive(Serialize, Deserialize)]
pub enum ControllerToTasMessage {
    PlayFile(String),
    Stop,
    SkipTo(u32),
    PauseAt(u32),
    AdvanceFrame,
    TeleportToTick(u32),
    TraceOptions(TraceDrawOptions),
}

#[derive(Serialize, Deserialize)]
pub enum TasToControllerMessage {
    PlaybackState(PlaybackState),
    CurrentTick(u32),
    ParseErrors(Vec<String>),
    CarlInfo {
        pos: (f32, f32, f32),
        ang: (f32, f32),
    },
    /// Indicates that a puzzle unlocked on the given tick
    PuzzleUnlock(u32),
}

/// Starts a server and listens for connections.
///
/// If the server is successfully opened, the channels never
/// hang up, so it should be safe to unwrap.
pub fn server_thread(
    sender: Sender<ControllerToTasMessage>,
    reciever: Receiver<TasToControllerMessage>,
) {
    let listener = match TcpListener::bind("127.0.0.1:7878") {
        Ok(listener) => listener,
        Err(err) => {
            error!("Error while opening port: {err}");
            return;
        }
    };

    let (send_streams, recv_streams) = channel();

    // Sender thread
    std::thread::spawn(move || {
        let mut streams: Vec<TcpStream> = vec![];

        // Send any messages
        for msg in reciever.iter() {
            for stream in recv_streams.try_iter() {
                streams.push(stream)
            }

            for stream in streams.iter_mut() {
                let json = serde_json::to_string(&msg).unwrap();
                if let Err(err) = stream.write_all(json.as_bytes()) {
                    error!("Error while sending message to controller: {err}")
                }
            }
        }

        reciever
    });

    for stream in listener.incoming() {
        let stream = stream.unwrap();
        send_streams.send(stream.try_clone().unwrap()).unwrap();

        // Attempt to get messages
        for msg in serde_json::Deserializer::from_reader(stream.try_clone().unwrap()).into_iter() {
            match msg {
                Ok(msg) => {
                    sender.send(msg).unwrap();
                }
                Err(err) => error!("Error while reading message from controller: {err}"),
            }
        }
    }
}

/// Connect to a tas server
///
/// Fail as soon as the stream dies (aka the game closed)
pub fn client_thread(
    sender: Sender<TasToControllerMessage>,
    reciever: Receiver<ControllerToTasMessage>,
) -> Result<(), std::io::Error> {
    // Attempt to connect to server
    let stream = TcpStream::connect("127.0.0.1:7878")?;
    let mut stream_copy = stream.try_clone().unwrap();

    std::thread::spawn(move || {
        // Forward any messages to the tas server
        for msg in reciever.iter() {
            let json = serde_json::to_string(&msg).unwrap();
            if let Err(err) = stream_copy.write_all(json.as_bytes()) {
                error!("Error while sending message to server: {err}");
                return;
            }
        }
    });

    // Attempt to get messages
    for msg in serde_json::Deserializer::from_reader(stream.try_clone().unwrap()).into_iter() {
        sender.send(msg?).unwrap();
    }

    Ok(())
}
