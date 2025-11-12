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
cargo run --release
```

The client will prompt you to enter the server's IP address. Copy the IP address from the server's console output and paste it into the client prompt, then press Enter.

### 3. Play the Game

The game window will open, and you can start playing. The client captures your keyboard input, sends it to the server, and the server sends back the updated game state to be rendered.

## Select Map

A set of map files in toml format is stored in the folder maps/. Choose one from 1 to 3 by using the flag '--map' or '-m' followed by a number. The game will default to map 1 if no choice is made.

```bash
cargo run --release --bin server -- --map 3
```
or
```bash
cargo run --release --bin server -- -m 3
```

You can also select your very own map. If you create a .toml file using the same format as other map files, and add it to the maps dir in root, you can play it using:
```bash
cargo run --release --bin server -- --map your_map_name_here
```
or
```bash
cargo run --release --bin server -- -m your_map_name_here
```

## Controls

- **WASD Keys:** Move
- **Mouse:** Turn
- **Space:** Jump
- **Shift:** Sprint
- **Arrow Keys**: Simple movement
- **Escape:** Exit the game
- **Tab:** Unlock and lock cursor