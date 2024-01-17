use enigo::*;
use futures_util::stream::StreamExt;
use serde::Deserialize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};
use warp::Filter;

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
enum ClientEvent {
    MouseMove {
        dx: f64,
        dy: f64,
        sx: f64,
        sy: f64,
        touches: i32,
    },
    MouseClick {
        button: MouseButton,
    },
    KeyPress {
        key: char,
    },
}

#[derive(Deserialize, Debug)]
enum MouseButton {
    Left,
    Right,
}

fn current_time_millis() -> u128 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_millis(),
        Err(_) => 0,
    }
}

fn should_process_scroll_message(last_processed_time: &Arc<AtomicU64>, time_interval: u64) -> bool {
    let now = current_time_millis() as u64;
    let last_time = last_processed_time.load(Ordering::Relaxed);
    let elapsed = now - last_time;
    if elapsed >= time_interval {
        last_processed_time.store(now, Ordering::Relaxed);
        true
    } else {
        false
    }
}

fn process_mouse_events(
    receiver: mpsc::Receiver<ClientEvent>,
    last_processed_time: Arc<AtomicU64>,
) {
    let mut enigo = Enigo::new();

    while let Ok(event) = receiver.recv() {
        match event {
            ClientEvent::MouseMove {
                dx,
                dy,
                sx,
                sy,
                touches,
            } => {
                let mut dx = dx;
                let mut dy = dy;

                if touches == 2 {
                    let scroll_factor = 10;
                    let dy_int = dy.round() as i32;
                    let mut scroll_lines = dy_int / scroll_factor;

                    if dy_int != 0 && scroll_lines == 0 {
                        scroll_lines = if dy_int > 0 { 1 } else { -1 };
                    }

                    if scroll_lines != 0 && should_process_scroll_message(&last_processed_time, 100)
                    {
                        enigo.mouse_scroll_y(scroll_lines);
                        println!("Mouse scrolled by: dy={}", scroll_lines);
                    }

                    continue;
                }

                // Do not respond to move messages for a period of time after scrolling
                if should_process_scroll_message(&last_processed_time, 1000) {
                    continue;
                }

                // Calculate the acceleration based on speed and distance
                // and adjust the mouse movement accordingly
                let acceleration_factor = 10.0; // Acceleration factor, adjustable according to actual requirements
                let distance = (dx.powi(2) + dy.powi(2)).sqrt();
                if distance > 1.0 {
                    let acceleration = distance * acceleration_factor;
                    dx += sx * acceleration;
                    dy += sy * acceleration;
                }

                let dx_int = dx.round() as i32;
                let dy_int = dy.round() as i32;

                // Discard abnormal movement distances
                if dx_int >= 1000 || dy_int >= 1000 {
                    continue;
                }

                enigo.mouse_move_relative(dx_int, dy_int);
                println!("Mouse moved by: dx={}, dy={}", dx_int, dy_int);
            }
            ClientEvent::MouseClick { button } => {
                match button {
                    MouseButton::Left => enigo.mouse_click(enigo::MouseButton::Left),
                    MouseButton::Right => enigo.mouse_click(enigo::MouseButton::Right),
                }
                println!("Mouse button clicked: {:?}", button);
            }
            ClientEvent::KeyPress { key } => {
                enigo.key_click(Key::Layout(key));
                println!("Key pressed: {}", key);
            }
        }
    }
    println!("Mouse event thread is terminating due to the closing of the channel.");
}

async fn handle_websocket(
    socket: warp::ws::WebSocket,
    mouse_event_sender: mpsc::Sender<ClientEvent>,
) {
    let (_ws_tx, mut ws_rx) = socket.split();

    while let Some(message_result) = ws_rx.next().await {
        match message_result {
            Ok(msg) => {
                if let Ok(text) = msg.to_str() {
                    if let Ok(mouse_move) = serde_json::from_str::<ClientEvent>(text) {
                        if mouse_event_sender.send(mouse_move).is_err() {
                            eprintln!("Failed to send mouse event; terminating connection.");
                            break;
                        }
                    } else {
                        eprintln!("Failed to parse mouse movement data. {}", text);
                    }
                }
            }
            Err(e) => {
                eprintln!("WebSocket receive error: {}", e);
                break;
            }
        }
    }
    println!("WebSocket connection closed.");
}

#[tokio::main]
async fn main() {
    let last_processed_time = Arc::new(AtomicU64::new(0));

    let (mouse_event_sender, mouse_event_receiver) = mpsc::channel::<ClientEvent>();

    thread::spawn(move || {
        process_mouse_events(mouse_event_receiver, last_processed_time);
    });

    let static_files = warp::fs::dir("public");

    let mouse_event_sender_filter = warp::any().map(move || mouse_event_sender.clone());
    let websocket_route = warp::path("ws")
        .and(warp::ws())
        .and(mouse_event_sender_filter)
        .map(|ws: warp::ws::Ws, sender| {
            ws.on_upgrade(move |socket| handle_websocket(socket, sender))
        });

    let routes = static_files.or(websocket_route);

    warp::serve(routes).run(([0, 0, 0, 0], 8088)).await;
}
