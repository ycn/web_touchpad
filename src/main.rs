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
        width: f64,
        height: f64,
        x: f64,
        y: f64,
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

fn process_mouse_events(
    receiver: mpsc::Receiver<ClientEvent>,
    last_processed_time: Arc<AtomicU64>, // Keep this for scroll throttling
) {
    let mut enigo = Enigo::new();
    let mut prev_dx = 0.0;
    let mut prev_dy = 0.0;

    // --- Constants for Tuning ---
    // Inertia/Coasting (Simplified: applied during movement, not after lifting finger)
    // A smaller factor reduces the 'drag' effect during movement. True coasting needs more state.
    const INERTIA_FACTOR: f64 = 0.08; // Try values between 0.0 and 0.4

    // Acceleration Curve (Speed-based)
    const MIN_SPEED_FOR_ACCEL: f64 = 0.7; // Speed threshold (pixels/event time) to start acceleration
    const ACCEL_POWER: f64 = 1.4;        // How aggressively acceleration ramps up with speed (try 1.2-1.8)
    const ACCEL_MULTIPLIER: f64 = 1.05;   // Overall acceleration strength (try 0.5-1.5)

    // Precision Mode (Low Speed)
    const MAX_SPEED_FOR_PRECISION: f64 = 0.7; // Speed threshold for precision mode
    const PRECISION_FACTOR: f64 = 4.0;       // Sensitivity reduction (try 0.3-0.7)

    // Edge Damping
    const EDGE_ZONE_PX: f64 = 40.0;          // Pixel distance from edge to start damping
    const EDGE_DAMPING_FACTOR: f64 = 0.7;    // Damping multiplier near edge (try 0.5-0.8)

    // Scroll Tuning
    const SCROLL_BASE_FACTOR: f64 = 2.5;     // Base sensitivity for scrolling (try 1.5-4.0)
    const SCROLL_ACCEL_FACTOR: f64 = 0.15;   // Acceleration based on scroll delta (try 0.1-0.3)
    const SCROLL_INTERVAL_MS: u64 = 25;      // Throttle scroll events (milliseconds)
    // Removed explicit post-scroll movement delay, relying on scroll throttling

    let mut last_scroll_time = 0u64; // Track last scroll event time separately

    while let Ok(event) = receiver.recv() {
        match event {
            ClientEvent::MouseMove {
                dx,
                dy,
                sx,
                sy,
                touches,
                width,
                height,
                x,
                y,
            } => {
                let mut current_dx = dx;
                let mut current_dy = dy;
                let now = current_time_millis() as u64;

                // --- Scrolling Logic (2 touches) ---
                if touches == 2 {
                    // Use dy for vertical scroll speed, sx could potentially be used for horizontal later
                    let scroll_speed_input = dy; // Use raw dy for speed basis
                    let scroll_speed = scroll_speed_input * SCROLL_BASE_FACTOR;
                    // Acceleration based on the magnitude of the scroll input
                    let scroll_accel = (scroll_speed_input.abs() * SCROLL_ACCEL_FACTOR).min(2.0); // Cap acceleration effect
                    let scroll_value = (scroll_speed * (1.0 + scroll_accel)).round() as i32;

                    // Throttle scroll events based on SCROLL_INTERVAL_MS
                    if scroll_value != 0 && (now - last_scroll_time >= SCROLL_INTERVAL_MS) {
                        enigo.mouse_scroll_y(-scroll_value); // Negative for natural scrolling
                        println!("Scroll: dy={}, val={}", dy, -scroll_value);
                        last_scroll_time = now; // Update time of last processed scroll
                        last_processed_time.store(now, Ordering::Relaxed); // Also update general time
                    }
                    // Reset movement inertia when scrolling
                    prev_dx = 0.0;
                    prev_dy = 0.0;
                    continue; // Don't process movement if scrolling
                }

                // --- Movement Logic (1 touch or default) ---
                let speed = (sx.powi(2) + sy.powi(2)).sqrt();

                // 1. Precision Mode (Low Speed)
                if speed < MAX_SPEED_FOR_PRECISION {
                    current_dx *= PRECISION_FACTOR;
                    current_dy *= PRECISION_FACTOR;
                    // Reset inertia in precision mode for responsiveness
                    prev_dx = 0.0;
                    prev_dy = 0.0;
                }
                // 2. Acceleration (Higher Speed)
                else if speed > MIN_SPEED_FOR_ACCEL {
                    // Calculate acceleration based on how much speed exceeds the minimum
                    let speed_excess = (speed - MIN_SPEED_FOR_ACCEL).max(0.0);
                    // Apply a non-linear acceleration curve
                    let acceleration_factor = 1.0 + ACCEL_MULTIPLIER * speed_excess.powf(ACCEL_POWER);
                    current_dx *= acceleration_factor;
                    current_dy *= acceleration_factor;
                }
                // Else (Medium Speed): No precision adjustment, no acceleration (base sensitivity)

                // 3. Edge Damping (Apply after acceleration/precision)
                if x < EDGE_ZONE_PX || x > width - EDGE_ZONE_PX || y < EDGE_ZONE_PX || y > height - EDGE_ZONE_PX {
                    current_dx *= EDGE_DAMPING_FACTOR;
                    current_dy *= EDGE_DAMPING_FACTOR;
                    // Reduce inertia buildup near edges
                    prev_dx *= EDGE_DAMPING_FACTOR;
                    prev_dy *= EDGE_DAMPING_FACTOR;
                }

                // 4. Inertia (Apply simplified inertia based on previous frame's output)
                current_dx += prev_dx * INERTIA_FACTOR;
                current_dy += prev_dy * INERTIA_FACTOR;

                // 5. Final Calculations & Output
                let dx_int = current_dx.round() as i32;
                let dy_int = current_dy.round() as i32;

                // Store the calculated delta *before* rounding for potentially smoother inertia next frame
                prev_dx = current_dx;
                prev_dy = current_dy;

                // Discard abnormal movement distances (potential jumps)
                if dx_int.abs() >= 1000 || dy_int.abs() >= 1000 {
                    println!("Discarding abnormal move: dx={}, dy={}", dx_int, dy_int);
                    prev_dx = 0.0; // Reset inertia on abnormal jump
                    prev_dy = 0.0;
                    continue;
                }

                // Only move if there's a change
                if dx_int != 0 || dy_int != 0 {
                    enigo.mouse_move_relative(dx_int, dy_int);
                    // println!("Move: dx={}, dy={} (Speed: {:.2})", dx_int, dy_int, speed); // Optional debug log
                    last_processed_time.store(now, Ordering::Relaxed); // Update last processed time
                } else {
                    // If movement rounded to zero, significantly decay inertia
                    prev_dx *= 0.5;
                    prev_dy *= 0.5;
                }
            }
            ClientEvent::MouseClick { button } => {
                // Reset inertia completely on click
                prev_dx = 0.0;
                prev_dy = 0.0;
                match button {
                    MouseButton::Left => enigo.mouse_click(enigo::MouseButton::Left),
                    MouseButton::Right => enigo.mouse_click(enigo::MouseButton::Right),
                }
                println!("Click: {:?}", button);
                last_processed_time.store(current_time_millis() as u64, Ordering::Relaxed);
            }
            ClientEvent::KeyPress { key } => {
                // Typically, don't reset inertia on key press while potentially moving
                enigo.key_click(Key::Layout(key));
                println!("Key: {}", key);
                last_processed_time.store(current_time_millis() as u64, Ordering::Relaxed);
            }
        }
    }
    println!("Mouse event thread terminated.");
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
