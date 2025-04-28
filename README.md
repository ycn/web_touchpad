# Web Touchpad

[![Rust](https://github.com/ycn/web_touchpad/actions/workflows/pub.yml/badge.svg)](https://github.com/ycn/web_touchpad/actions/workflows/pub.yml)

Turn any device with a web browser into a wireless touchpad and keyboard for your computer.

This application runs a lightweight web server on your host machine. Connecting to it from another device on the same network (like a phone or tablet) presents a web interface that functions as a remote touchpad.

## Features

*   **Remote Mouse Control:** Smooth mouse movement simulation.
*   **Click Support:** Left and Right mouse clicks.
*   **Two-Finger Scrolling:** Natural vertical scrolling.
*   **Basic Keyboard Input:** Send keystrokes from the web interface.
*   **Adjustable Sensitivity & Acceleration:** Fine-tune pointer speed, acceleration curves, precision mode, inertia, and edge damping via constants in the source code (`src/main.rs`) for a personalized feel.
*   **Cross-Platform:** Runs on Windows, macOS, and Linux (wherever Rust and `enigo` are supported).
*   **Zero Client Installation:** Only requires a modern web browser on the remote device.

## Prerequisites

*   **Rust:** Install Rust and Cargo (The Rust toolchain) from [https://rustup.rs/](https://rustup.rs/).

## Building

1.  Clone the repository:
    ```bash
    git clone https://github.com/ycn/web_touchpad.git
    cd web_touchpad
    ```
2.  Build the release executable:
    ```bash
    cargo build --release
    ```
    The executable will be located at `target/release/web_touchpad` (or `target\release\web_touchpad.exe` on Windows).

## Running

1.  **Copy Static Files:** Ensure the `public` directory (containing `index.html`, CSS, and JS files) is placed in the **same directory** as the compiled executable (`web_touchpad` or `web_touchpad.exe`).
2.  **Run the Executable:**
    *   On Linux/macOS: `./target/release/web_touchpad`
    *   On Windows: `.\target\release\web_touchpad.exe`

    The server will start and listen on `0.0.0.0:8088` by default.

## Usage

1.  Find the **local IP address** of the computer running the `web_touchpad` server.
    *   macOS: `ipconfig getifaddr en0` (or `en1` for Wi-Fi) in the terminal.
    *   Linux: `ip addr show` in the terminal.
    *   Windows: `ipconfig` in Command Prompt.
2.  On your remote device (phone, tablet, another computer), open a web browser.
3.  Navigate to `http://<SERVER_IP_ADDRESS>:8088` (replace `<SERVER_IP_ADDRESS>` with the actual IP address you found).
4.  The web touchpad interface should load. Use it to control the mouse and keyboard of the server computer.

## Configuration / Tuning

Pointer behavior (acceleration, precision, inertia, scrolling) can be fine-tuned by modifying the `const` values within the `process_mouse_events` function in `src/main.rs`. Recompile the application after making changes.

## Contributing

Contributions are welcome! Please feel free to submit pull requests or open issues.

## License

This project is licensed under the **MIT License** - see the [LICENSE](LICENSE) file for details.
