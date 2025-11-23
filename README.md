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

## Server Options

The server supports several command-line flags to customize map selection and game behavior.

### Select a Specific Map

A set of map files in toml format is stored in the folder maps/. Choose one from 1 to 3 by using the flag `--map` or `-m` followed by a number. The game will default to map 1 if no choice is made.

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

### Use a Random Premade Map

If you don't specify a map, the server will randomly select one of the premade maps (1-3) for each new game round. This is the default behavior when no map flags are provided.

### Use a Randomly Generated Map

Use the `--random-map` or `-rm` flag to have the server generate a completely random map for each game round instead of using a premade map.

```bash
cargo run --release --bin server -- --random-map
```
or
```bash
cargo run --release --bin server -- -rm
```

**Note:** You cannot use both `--map` and `--random-map` at the same time.

### Keep Map Between Matches

Use the `--permanent-map` or `-pm` flag to keep the same map across all game rounds. Without this flag, the server will select a new map (or generate a new random map) for each new game round.

```bash
cargo run --release --bin server -- --permanent-map
```
or
```bash
cargo run --release --bin server -- -pm
```

You can combine `--permanent-map` with `--map` to use a specific map that persists across rounds:

```bash
cargo run --release --bin server -- --map 2 --permanent-map
```

## Controls

- **WASD Keys:** Move
- **Mouse:** Turn
- **Space:** Jump
- **Shift:** Sprint
- **Arrow Keys**: Simple movement
- **Escape:** Exit the game
- **Tab:** Unlock and lock cursor