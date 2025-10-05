# FPS Game

A simple first-person game built with Rust, featuring a client-server architecture using UDP.

## How to Run

To play the game, you need to run both the server and the client in separate terminal windows.

### 1. Start the Server

In your first terminal, run the following command to start the game server:

```bash
cargo run --release --bin server
```

The server will start and print its local IP address to the console. It will look something like this:

```
Server started at 192.168.1.10:8080
```

### 2. Start the Client

In your second terminal, run this command to start the client:

```bash
cargo run --release --bin client
```

The client will prompt you to enter the server's IP address. Copy the IP address from the server's console output and paste it into the client prompt, then press Enter.

### 3. Play the Game

The game window will open, and you can start playing. The client captures your keyboard input, sends it to the server, and the server sends back the updated game state to be rendered.

## Controls

-   **Up Arrow:** Move forward
-   **Down Arrow:** Move backward
-   **Left Arrow:** Rotate left
-   **Right Arrow:** Rotate right
-   **Alt/Shift + Left/Right Arrow:** Strafe left/right
-   **Escape:** Exit the game